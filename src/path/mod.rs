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

    pub fn push(&mut self, segment: Segment) {
        self.segments.push(segment);
    }

    pub fn push_filter(&mut self, key: &str, value: &str) {
        self.segments
            .push(Segment::Filter(vec![(key.to_owned(), value.to_owned())]));
    }

    pub fn pop(&mut self) -> Option<Segment> {
        self.segments.pop()
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
                    path_str.push('[');
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
