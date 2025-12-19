use serde_json::Value;

use crate::{
    patch::{add, error::PatchError},
    path::Spath,
    resolve::resolve_ref,
};

/// The "copy" operation copies the value at a specified location to the
/// target location.
///
/// The operation object MUST contain a "from" member, which is a string
/// containing a JSON Pointer value that references the location in the
/// target document to copy the value from.
///
/// The "from" location MUST exist for the operation to be successful.
///
/// For example:
///
/// { "op": "copy", "from": "/a/b/c", "path": "/a/b/e" }
///
/// This operation is functionally identical to an "add" operation at the
/// target location using the value specified in the "from" member.
pub fn copy(doc: &mut Value, from: Spath, path: Spath) -> Result<(), PatchError> {
    let value = resolve_ref(doc, &from)?.clone();

    add(doc, path, value)
}

#[cfg(test)]
mod tests {
    use assert2::{check, let_assert};
    use serde_json::json;

    use crate::resolve::ResolveError;

    use super::*;

    #[test]
    fn copy_from_nonexistent_path_should_fail() {
        let mut doc = json!({"a": 1, "b": 2});

        let_assert!(
            Err(PatchError::ResolveError(ResolveError::NotFound)) =
                copy(&mut doc, "/c".try_into().unwrap(), "/d".try_into().unwrap())
        );
        check!(doc == json!({"a": 1, "b": 2}));
    }

    #[test]
    fn copy_root_to_child_should_succeed() {
        let mut doc = json!({"a": 1, "b": 2});

        let_assert!(Ok(()) = copy(&mut doc, "".try_into().unwrap(), "/c".try_into().unwrap()));
        check!(doc == json!({"a": 1, "b": 2, "c": {"a": 1, "b": 2}}));
    }

    #[test]
    fn copy_child_to_root_should_succeed() {
        let mut doc = json!({"a": 1, "b": 2});

        let_assert!(Ok(()) = copy(&mut doc, "/a".try_into().unwrap(), "".try_into().unwrap()));
        check!(doc == json!(1));
    }
}
