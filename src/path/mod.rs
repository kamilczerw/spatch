use std::fmt::Display;

use parser::parse_path;

mod parser;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Segment {
    /// Represents a field in the object.
    Field(String),

    /// Represents a filter for array elements.
    /// Key is the field name to filter on, and value is the expected value.
    Filter(Vec<(String, String)>),
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Spath {
    pub(crate) segments: Vec<Segment>,
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum PathError {
    #[error("Invalid path format")]
    InvalidFormat,

    #[error("Path cannot be empty")]
    EmptyPath,
}

impl TryFrom<&str> for Spath {
    type Error = PathError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        parse_path(value)
            .map_err(|err| match err {
                nom::Err::Error(_) | nom::Err::Failure(_) => PathError::InvalidFormat,
                nom::Err::Incomplete(_) => PathError::InvalidFormat,
            })
            .map(|(_, spath)| spath)
    }
}

impl IntoIterator for Spath {
    type Item = Segment;
    type IntoIter = std::vec::IntoIter<Segment>;

    fn into_iter(self) -> Self::IntoIter {
        self.segments.into_iter()
    }
}

impl Spath {
    pub fn is_empty(&self) -> bool {
        self.segments.is_empty()
    }

    pub fn push(&self, segment: Segment) -> Self {
        let mut segments = self.segments.clone();
        segments.push(segment);
        Spath { segments }
    }

    pub fn push_filter(&self, key: &str, value: &str) -> Self {
        let mut segments = self.segments.clone();
        segments.push(Segment::Filter(vec![(key.to_owned(), value.to_owned())]));

        Spath { segments }
    }
}

impl Display for Spath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut path_str = String::new();
        for segment in &self.segments {
            match segment {
                Segment::Field(field) => {
                    path_str.push('/');
                    path_str.push_str(field);
                }
                Segment::Filter(filters) => {
                    path_str.push_str("/[");
                    let filter_strs: Vec<String> = filters
                        .iter()
                        .map(|(k, v)| format!("{}={}", k, v))
                        .collect();
                    path_str.push_str(&filter_strs.join(","));
                    path_str.push(']');
                }
            }
        }
        write!(f, "{}", path_str)
    }
}

impl serde::Serialize for Spath {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

#[cfg(test)]
mod tests {
    use assert2::check;

    use super::*;

    #[test]
    fn test_spath_try_from_str() {
        let path_str = "/field1/field2/[filterKey=filterValue]/field3";
        let spath = Spath::try_from(path_str).unwrap();

        let expected_spath = Spath {
            segments: vec![
                Segment::Field("field1".to_string()),
                Segment::Field("field2".to_string()),
                Segment::Filter(vec![("filterKey".to_string(), "filterValue".to_string())]),
                Segment::Field("field3".to_string()),
            ],
        };

        check!(spath == expected_spath);
    }

    #[test]
    fn test_spath_try_from_with_invalid_format_should_fail() {
        check!(Spath::try_from("/foo[bar=baz]/field3") == Err(PathError::InvalidFormat));
        check!(Spath::try_from("/foo[bar=baz") == Err(PathError::InvalidFormat));
        check!(Spath::try_from("/fooba//rbaz") == Err(PathError::InvalidFormat));
        check!(Spath::try_from("fooba/rbaz") == Err(PathError::InvalidFormat));
    }

    #[test]
    fn test_spath_display() {
        let spath = Spath {
            segments: vec![
                Segment::Field("field1".to_string()),
                Segment::Field("field2".to_string()),
                Segment::Filter(vec![("filterKey".to_string(), "filterValue".to_string())]),
                Segment::Field("field3".to_string()),
            ],
        };

        let path_str = spath.to_string();

        check!(path_str == "/field1/field2/[filterKey=filterValue]/field3");
    }
}
