/// Configuration for [`diff`](crate::diff::diff).
///
/// `DiffOptions` lets you choose how patches should read to humans without
/// giving up the JSON Patch wire format. Use it to opt into schema-aware array
/// paths, choose compact output for small payloads, or choose granular output
/// for review-friendly patches.
///
/// # Schema-aware array paths
///
/// When a schema marks an array with `indexKey`, spatch can address items by
/// identity instead of by position. That keeps patches stable when arrays are
/// reordered, prepended to, or trimmed.
///
/// ```rust
/// use serde_json::json;
/// use spatch::diff::{diff, DiffOptions};
///
/// let schema = json!({
///     "properties": {
///         "users": {
///             "indexKey": "id",
///             "items": {
///                 "properties": {
///                     "name": {}
///                 }
///             }
///         }
///     }
/// });
///
/// let before = json!({"users": [{"id": "u-1", "name": "Ada"}]});
/// let after = json!({"users": [{"id": "u-1", "name": "Ada Lovelace"}]});
///
/// let patch = diff(&before, &after, DiffOptions::new().with_schema(&schema)).unwrap();
/// let patch_json = serde_json::to_value(&patch).unwrap();
///
/// assert_eq!(patch_json[0]["path"], "/users/[id=u-1]/name");
/// ```
///
/// # Granularity
///
/// The default [`DiffGranularity::Compact`] mode may replace a whole object
/// when that is smaller than many nested operations. Switch to
/// [`DiffGranularity::Granular`] when you want patches that are easier to read,
/// review, and explain.
///
/// ```rust
/// use serde_json::json;
/// use spatch::diff::{diff, DiffOptions};
///
/// let before = json!({"profile": {"name": "Ada", "city": "London"}});
/// let after = json!({"profile": {"name": "Ada", "city": "Oxford"}});
///
/// let patch = diff(&before, &after, DiffOptions::new().granular()).unwrap();
/// let patch_json = serde_json::to_value(&patch).unwrap();
///
/// assert_eq!(patch_json[0]["path"], "/profile/city");
/// ```
#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub struct DiffOptions<'a> {
    /// Optional schema used to discover semantic array identity rules.
    ///
    /// spatch looks for the custom `indexKey` property on array schemas. When
    /// present, array elements are diffed and emitted as semantic paths such as
    /// `/items/[id=item-42]` instead of index paths such as `/items/0`.
    pub schema: Option<&'a serde_json::Value>,

    /// Controls whether object diffs prefer smaller patches or nested patches.
    pub granularity: DiffGranularity,
}

/// Controls how aggressively spatch collapses object changes.
///
/// Both modes produce valid JSON Patch operations. The choice is about the
/// shape of the patch:
///
/// - [`Compact`](Self::Compact) is optimized for smaller serialized patches.
/// - [`Granular`](Self::Granular) is optimized for readability and review.
///
/// ```rust
/// use serde_json::json;
/// use spatch::diff::{diff, DiffOptions};
///
/// let before = json!({"settings": {"a": 1, "b": 2, "c": 3}});
/// let after = json!({"settings": {"a": 10, "b": 20, "c": 30}});
///
/// let compact = diff(&before, &after, DiffOptions::new().compact()).unwrap();
/// let granular = diff(&before, &after, DiffOptions::new().granular()).unwrap();
///
/// assert!(compact.len() <= granular.len());
/// ```
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
    /// Creates default diff options.
    ///
    /// Defaults are intentionally conservative and convenient:
    ///
    /// - no schema, so arrays use standard index-based JSON Patch paths;
    /// - [`DiffGranularity::Compact`], so large object changes can collapse to
    ///   a smaller parent `replace` operation.
    pub fn new() -> Self {
        Self::default()
    }

    /// Enables schema-aware diffing.
    ///
    /// The schema is borrowed, so callers can keep schema loading and caching
    /// outside the diff engine. spatch currently recognizes `indexKey` on array
    /// schemas to select the string property that identifies array elements.
    ///
    /// ```rust
    /// use serde_json::json;
    /// use spatch::diff::{diff, DiffOptions};
    ///
    /// let schema = json!({
    ///     "properties": {
    ///         "todos": { "indexKey": "id" }
    ///     }
    /// });
    ///
    /// let before = json!({"todos": [{"id": "t-1", "done": false}]});
    /// let after = json!({"todos": [{"id": "t-1", "done": true}]});
    ///
    /// let patch = diff(&before, &after, DiffOptions::new().with_schema(&schema).granular())
    ///     .unwrap();
    /// let patch_json = serde_json::to_value(&patch).unwrap();
    ///
    /// assert_eq!(patch_json[0]["path"], "/todos/[id=t-1]/done");
    /// ```
    pub fn with_schema(mut self, schema: &'a serde_json::Value) -> Self {
        self.schema = Some(schema);
        self
    }

    /// Clears the active schema.
    ///
    /// This is useful when reusing an options builder but intentionally falling
    /// back to pure RFC 6902, index-based diffing.
    pub fn without_schema(mut self) -> Self {
        self.schema = None;
        self
    }

    /// Sets the diff granularity explicitly.
    pub fn with_granularity(mut self, granularity: DiffGranularity) -> Self {
        self.granularity = granularity;
        self
    }

    /// Chooses review-friendly object diffs.
    ///
    /// In granular mode, spatch keeps walking into objects and emits nested
    /// operations instead of replacing a parent object just because that parent
    /// replacement is shorter on the wire.
    pub fn granular(mut self) -> Self {
        self.granularity = DiffGranularity::Granular;
        self
    }

    /// Chooses compact object diffs.
    ///
    /// Compact mode is the default. When a parent object replacement serializes
    /// smaller than many child operations, spatch emits the parent `replace`.
    pub fn compact(mut self) -> Self {
        self.granularity = DiffGranularity::Compact;
        self
    }

    /// Sets or clears the active schema in one call.
    ///
    /// Passing `Some(schema)` behaves like [`with_schema`](Self::with_schema).
    /// Passing `None` behaves like [`without_schema`](Self::without_schema) and
    /// intentionally prevents a parent schema from leaking into child values
    /// that do not have their own schema entry.
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
