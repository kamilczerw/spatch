#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum PatchError {
    #[error("Failed to resolve path: {0}")]
    ResolveError(#[from] crate::resolve::ResolveError),

    #[error("TODO: implement PatchError variant")]
    TODO,

    #[error("Cannot remove the root of the document")]
    CannotRemoveRoot,

    #[error("Cannot move a value into one of its children")]
    CannotMoveIntoChild,

    #[error("The values at the source and target paths are not equal")]
    ValuesNotEqual,
}
