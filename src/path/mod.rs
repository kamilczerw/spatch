mod error;
mod parser;

use std::fmt::Display;

pub use crate::path::error::PathError;

use parser::parse_path;

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

impl TryFrom<&str> for Spath {
    type Error = PathError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        if value.is_empty() {
            return Ok(Spath::default());
        }

        #[allow(clippy::redundant_guards)] // Cleaner to have a guard instead of nesting the if
        match parse_path(value) {
            Ok((rest, spath)) if rest.is_empty() => Ok(spath),

            Ok((rest, _)) => {
                // Parsed a valid prefix but there's junk left.
                Err(error::trailing_input_error(value, rest))
            }
            Err(nom::Err::Error(e)) | Err(nom::Err::Failure(e)) => {
                Err(error::convert_verbose_error(value, e))
            }

            Err(nom::Err::Incomplete(_)) => Err(PathError::InvalidSyntax {
                position: value.len(),
                message: "unexpected end of input".into(),
            }),
        }
    }
}

impl IntoIterator for Spath {
    type Item = Segment;
    type IntoIter = std::vec::IntoIter<Segment>;

    fn into_iter(self) -> Self::IntoIter {
        self.segments.into_iter()
    }
}

impl<'a> IntoIterator for &'a Spath {
    type Item = &'a Segment;
    type IntoIter = std::slice::Iter<'a, Segment>;

    fn into_iter(self) -> Self::IntoIter {
        self.segments.iter()
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

    /// Returns a parent path, or None if there is no parent.
    pub fn parent(&self) -> Option<Spath> {
        if self.segments.is_empty() {
            None
        } else {
            let segments = self.segments[..self.segments.len() - 1].to_vec();
            Some(Spath { segments })
        }
    }

    pub fn field(&self) -> Option<String> {
        self.segments.last().and_then(|segment| {
            if let Segment::Field(field) = segment {
                Some(field.clone())
            } else {
                None
            }
        })
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

    use crate::path::error::UNEXPECTED_SQ_BRACKET_MSG;

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
        check!(
            Spath::try_from("/foo[bar=baz]/field3")
                == Err(PathError::invalid_syntax(4, UNEXPECTED_SQ_BRACKET_MSG))
        );
        check!(
            Spath::try_from("/foo[bar=baz")
                == Err(PathError::invalid_syntax(4, UNEXPECTED_SQ_BRACKET_MSG))
        );
        check!(
            Spath::try_from("fooba/rbaz")
                == Err(PathError::invalid_syntax(
                    0,
                    "expected a path starting with '/' or empty input"
                ))
        );
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

    #[test]
    fn spath_parent_should_return_parent_path() {
        let spath = Spath {
            segments: vec![
                Segment::Field("a".to_string()),
                Segment::Field("b".to_string()),
                Segment::Field("c".to_string()),
            ],
        };

        let parent = spath.parent().unwrap();

        let expected_parent = Spath {
            segments: vec![
                Segment::Field("a".to_string()),
                Segment::Field("b".to_string()),
            ],
        };

        check!(parent == expected_parent);
    }

    #[test]
    fn spath_parent_of_root_should_be_none() {
        let spath = Spath { segments: vec![] };

        let parent = spath.parent();

        check!(parent == None);
    }
}
