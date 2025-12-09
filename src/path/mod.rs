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

pub struct Spath {
    pub(crate) segments: Vec<Segment>,
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum SpathError {
    #[error("Invalid path format")]
    InvalidFormat,

    #[error("Path cannot be empty")]
    EmptyPath,
}

impl TryFrom<&str> for Spath {
    type Error = SpathError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        parse_path(value)
            .map_err(|err| match err {
                nom::Err::Error(_) | nom::Err::Failure(_) => SpathError::InvalidFormat,
                nom::Err::Incomplete(_) => SpathError::InvalidFormat,
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
