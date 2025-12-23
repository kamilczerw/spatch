use crate::path::Spath;

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum PatchError {
    #[error("Failed to resolve path: {0}")]
    ResolveError(#[from] crate::resolve::ResolveError),

    #[error("Parent for path {path} does not exist")]
    MissingParent { path: Spath },

    #[error("Missing final token for path {path}")]
    MissingFinalToken { path: Spath },

    #[error("Cannot add into non-container at {parent}: expected object or array, found {actual}")]
    NotAContainer { parent: Spath, actual: String },

    // Array-specific errors.
    #[error("Invalid array index token '{token}' at path {path}")]
    InvalidArrayIndexToken { path: Spath, token: String },

    #[error("Array index {index} out of bounds at {path}: length is {len} (index must be <= len)")]
    ArrayIndexOutOfBounds {
        path: Spath,
        index: usize,
        len: usize,
    },

    #[error("Target not found at path {path}")]
    TargetNotFound { path: Spath },

    #[error("Cannot remove the root of the document")]
    CannotRemoveRoot,

    #[error("Cannot move a value into one of its children")]
    CannotMoveIntoChild,

    #[error("The values at the source and target paths are not equal")]
    ValuesNotEqual,

    #[error("Multiple errors occurred: {0:?}")]
    MultipleErrors(Vec<PatchError>),
}

impl PatchError {
    pub fn missing_parent(path: &Spath) -> Self {
        PatchError::MissingParent { path: path.clone() }
    }

    pub fn missing_final_token(path: &Spath) -> Self {
        PatchError::MissingFinalToken { path: path.clone() }
    }

    pub fn invalid_array_index_token(path: &Spath, token: &str) -> Self {
        PatchError::InvalidArrayIndexToken {
            path: path.clone(),
            token: token.to_string(),
        }
    }

    pub fn index_out_of_bounds(path: &Spath, index: usize, len: usize) -> Self {
        PatchError::ArrayIndexOutOfBounds {
            path: path.clone(),
            index,
            len,
        }
    }

    pub fn not_a_container(path: &Spath, actual: &str) -> Self {
        PatchError::NotAContainer {
            parent: path.clone(),
            actual: actual.to_string(),
        }
    }

    pub fn target_not_found(path: &Spath) -> Self {
        PatchError::TargetNotFound { path: path.clone() }
    }
}
