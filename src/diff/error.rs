use std::{fmt, ops::Add};

use crate::path::Spath;

#[derive(Debug, thiserror::Error, PartialEq, Eq, Clone)]
#[allow(clippy::enum_variant_names)]
pub enum DiffError {
    /// An array schema specified an `x-spatch-indexKey`, which requires object items, but
    /// an array item was not a JSON object.
    #[error(
        "Expected array items at path {path} to be objects for schema-aware diffing, but found {found}"
    )]
    NonObjectArrayItem { path: Spath, found: String },

    /// An array item did not contain the property named by the array schema's
    /// `x-spatch-indexKey`.
    #[error("Item {path} is missing index key '{index_key}'")]
    MissingIndexKey { path: Spath, index_key: String },

    /// An array item's `x-spatch-indexKey` value cannot be represented as a semantic path
    /// filter.
    ///
    /// Despite the historical variant name, strings, numbers, and booleans are
    /// accepted as semantic identity values. This error is returned for object,
    /// array, and `null` values.
    #[error("Item {path} has non-string index key: {index_key}")]
    NonStringIndexKey {
        path: Spath,
        index_key: serde_json::Value,
    },

    /// Two array items had the same representable `x-spatch-indexKey` value.
    #[error("Item {path} has duplicate index key '{index_key}' with value '{value}'")]
    DuplicateIndexKey {
        path: Spath,
        index_key: String,
        value: String,
    },
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct DiffErrorSummary {
    pub left: Vec<DiffError>,
    pub right: Vec<DiffError>,
}

impl DiffError {
    pub fn non_object_array_item(path: &Spath, item: &serde_json::Value) -> Self {
        DiffError::NonObjectArrayItem {
            path: path.clone(),
            found: json_value_type_name(item).to_string(),
        }
    }

    pub fn missing_index_key(path: &Spath, index_key: &str) -> Self {
        DiffError::MissingIndexKey {
            path: path.clone(),
            index_key: index_key.to_string(),
        }
    }

    pub fn non_string_index_key(path: &Spath, index_key: &serde_json::Value) -> Self {
        DiffError::NonStringIndexKey {
            path: path.clone(),
            index_key: index_key.clone(),
        }
    }

    pub fn duplicate_index_key(path: &Spath, index_key: &str, value: &str) -> Self {
        DiffError::DuplicateIndexKey {
            path: path.clone(),
            index_key: index_key.to_string(),
            value: value.to_string(),
        }
    }
}

fn json_value_type_name(value: &serde_json::Value) -> &'static str {
    match value {
        serde_json::Value::Null => "null",
        serde_json::Value::Bool(_) => "boolean",
        serde_json::Value::Number(_) => "number",
        serde_json::Value::String(_) => "string",
        serde_json::Value::Array(_) => "array",
        serde_json::Value::Object(_) => "object",
    }
}

impl DiffErrorSummary {
    pub fn new(left: Vec<DiffError>, right: Vec<DiffError>) -> Self {
        DiffErrorSummary { left, right }
    }

    pub fn empty() -> Self {
        DiffErrorSummary {
            left: Vec::new(),
            right: Vec::new(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.left.is_empty() && self.right.is_empty()
    }
}

impl fmt::Display for DiffErrorSummary {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(error) = self.left.first().or_else(|| self.right.first()) {
            write!(f, "{error}")
        } else {
            write!(
                f,
                "Diff errors occurred; left: {:?}, right: {:?}",
                self.left, self.right
            )
        }
    }
}

impl std::error::Error for DiffErrorSummary {}

impl Add for DiffErrorSummary {
    type Output = DiffErrorSummary;

    fn add(self, rhs: Self) -> Self::Output {
        let mut left = self.left;
        left.extend(rhs.left);
        let mut right = self.right;
        right.extend(rhs.right);
        DiffErrorSummary { left, right }
    }
}
