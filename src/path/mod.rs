mod parser;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Segment {
    /// Represents a field in the object.
    Field(String),

    /// Represents an index in an array.
    Index(usize),

    /// Represents a filter for array elements.
    /// Key is the field name to filter on, and value is the expected value.
    Filter(Vec<(String, String)>),
}

pub struct Spath {
    segments: Vec<Segment>,
}
