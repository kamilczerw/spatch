use std::ops::Add;

use crate::path::Spath;

#[derive(Debug, thiserror::Error, PartialEq, Eq, Clone)]
pub enum DiffError {
    #[error("Item {path} is missing index key '{index_key}'")]
    MissingIndexKey { path: Spath, index_key: String },

    #[error("Item {path} has non-string index key: {index_key}")]
    NonStringIndexKey {
        path: Spath,
        index_key: serde_json::Value,
    },

    #[error("Item {path} has duplicate index key '{index_key}' with value '{value}'")]
    DuplicateIndexKey {
        path: Spath,
        index_key: String,
        value: String,
    },
}

#[derive(Debug, PartialEq, Eq, Clone, thiserror::Error)]
#[error("Diff errors occurred; left: {left:?}, right: {right:?}")]
pub struct DiffErrorSummary {
    pub left: Vec<DiffError>,
    pub right: Vec<DiffError>,
}

impl DiffError {
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
