#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub struct DiffOptions<'a> {
    pub schema: Option<&'a serde_json::Value>,
    pub granularity: DiffGranularity,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum DiffGranularity {
    /// Prefer smaller patches, even if that means replacing a whole object
    /// instead of emitting many nested operations.
    Compact,

    /// Prefer preserving structure. Do not collapse object-level diffs into
    /// a parent `replace` merely because it is smaller.
    Granular,
}

impl<'a> DiffOptions<'a> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_schema(mut self, schema: &'a serde_json::Value) -> Self {
        self.schema = Some(schema);
        self
    }

    pub fn without_schema(mut self) -> Self {
        self.schema = None;
        self
    }

    pub fn with_granularity(mut self, granularity: DiffGranularity) -> Self {
        self.granularity = granularity;
        self
    }

    pub fn granular(mut self) -> Self {
        self.granularity = DiffGranularity::Granular;
        self
    }

    pub fn compact(mut self) -> Self {
        self.granularity = DiffGranularity::Compact;
        self
    }

    pub fn with_optional_schema(mut self, schema: Option<&'a serde_json::Value>) -> Self {
        self.schema = schema;
        self
    }
}

impl<'a> Default for DiffOptions<'a> {
    fn default() -> Self {
        Self {
            schema: None,
            granularity: DiffGranularity::Compact,
        }
    }
}
