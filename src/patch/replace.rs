use serde_json::Value;

use crate::{patch::error::PatchError, path::Spath, resolve::resolve_mut};

/// The "replace" operation replaces the value at the target location
/// with a new value.  The operation object MUST contain a "value" member
/// whose content specifies the replacement value.
///
/// The target location MUST exist for the operation to be successful.
///
/// For example:
///
/// { "op": "replace", "path": "/a/b/c", "value": 42 }
///
/// This operation is functionally identical to a "remove" operation for
/// a value, followed immediately by an "add" operation at the same
/// location with the replacement value.
pub fn replace(doc: &mut Value, path: Spath, value: Value) -> Result<(), PatchError> {
    if path.is_empty() {
        *doc = value;
        return Ok(());
    }

    let target = resolve_mut(doc, &path)?;
    *target = value;
    Ok(())
}

#[cfg(test)]
mod tests {
    use assert2::{check, let_assert};
    use serde_json::json;

    use crate::resolve::ResolveError;

    use super::*;

    #[test]
    fn replace_empty_path_should_replace_the_entire_document() {
        let mut doc = json!({"a": 1});

        let_assert!(Ok(()) = replace(&mut doc, "".try_into().unwrap(), json!(42)));
        check!(doc == json!(42));
    }

    #[test]
    fn replace_slash_path_should_replace_the_entire_document_at_empty_key() {
        let mut doc = json!({"": {"a": 1}});

        let_assert!(Ok(()) = replace(&mut doc, "/".try_into().unwrap(), json!(42)));
        check!(doc == json!({"": 42}));
    }

    #[test]
    fn replace_existing_field_should_succeed() {
        let mut doc = json!({"a": 1, "b": 2});

        let_assert!(Ok(()) = replace(&mut doc, "/a".try_into().unwrap(), json!(42)));
        check!(doc == json!({"a": 42, "b": 2}));
    }

    #[test]
    fn resolve_non_existing_field_should_fail() {
        let mut doc = json!({"a": 1, "b": 2});

        let_assert!(
            Err(PatchError::ResolveError(ResolveError::NotFound)) =
                replace(&mut doc, "/c".try_into().unwrap(), json!(42))
        );
        check!(doc == json!({"a": 1, "b": 2}));
    }

    #[test]
    fn replace_field_in_non_object_should_fail() {
        let mut doc = json!({"a": [1, 2, 3]});

        let_assert!(
            Err(PatchError::ResolveError(_)) =
                replace(&mut doc, "/a/b".try_into().unwrap(), json!(42))
        );
        check!(doc == json!({"a": [1, 2, 3]}));
    }

    #[test]
    fn replace_field_in_nested_object_should_succeed() {
        let mut doc = json!({"a": {"b": {"c": 1}}});

        let_assert!(Ok(()) = replace(&mut doc, "/a/b/c".try_into().unwrap(), json!(42)));
        check!(doc == json!({"a": {"b": {"c": 42}}}));
    }

    #[test]
    fn replace_array_element_should_succeed() {
        let mut doc = json!({"a": [1, 2, 3]});

        let_assert!(Ok(()) = replace(&mut doc, "/a/1".try_into().unwrap(), json!(42)));
        check!(doc == json!({"a": [1, 42, 3]}));
    }

    #[test]
    fn replace_array_element_out_of_bounds_should_fail() {
        let mut doc = json!({"a": [1, 2, 3]});

        let_assert!(
            Err(PatchError::ResolveError(ResolveError::NotFound)) =
                replace(&mut doc, "/a/5".try_into().unwrap(), json!(42))
        );
        check!(doc == json!({"a": [1, 2, 3]}));
    }

    #[test]
    fn replace_array_element_out_of_bounds_at_len_should_fail() {
        let mut doc = json!({"a": [1, 2, 3]});

        let_assert!(
            Err(PatchError::ResolveError(ResolveError::NotFound)) =
                replace(&mut doc, "/a/3".try_into().unwrap(), json!(42))
        );

        check!(doc == json!({"a": [1, 2, 3]}));
    }
    #[test]
    fn replace_field_with_filter_should_succeed() {
        let mut doc = json!({
            "items": [
                {"id": 1, "value": "a"},
                {"id": 2, "value": "b"},
                {"id": 3, "value": "c"}
            ]
        });

        let_assert!(
            Ok(()) = replace(
                &mut doc,
                "/items/[id=2]/value".try_into().unwrap(),
                json!("z")
            )
        );
        check!(
            doc == json!({
                "items": [
                    {"id": 1, "value": "a"},
                    {"id": 2, "value": "z"},
                    {"id": 3, "value": "c"}
                ]
            })
        );
    }

    #[test]
    fn replace_field_with_non_matching_filter_should_fail() {
        let mut doc = json!({
            "items": [
                {"id": 1, "value": "a"},
                {"id": 2, "value": "b"},
                {"id": 3, "value": "c"}
            ]
        });

        let_assert!(
            Err(PatchError::ResolveError(ResolveError::NotFound)) = replace(
                &mut doc,
                "/items/[id=4]/value".try_into().unwrap(),
                json!("z")
            )
        );
        check!(
            doc == json!({
                "items": [
                    {"id": 1, "value": "a"},
                    {"id": 2, "value": "b"},
                    {"id": 3, "value": "c"}
                ]
            })
        );
    }

    #[test]
    fn replace_array_dash_should_fail() {
        let mut doc = json!({"a":[1,2,3]});
        let_assert!(
            Err(PatchError::ResolveError(_)) =
                replace(&mut doc, "/a/-".try_into().unwrap(), json!(42))
        );
        check!(doc == json!({"a":[1,2,3]}));
    }

    #[test]
    fn replace_array_non_numeric_index_should_fail() {
        let mut doc = json!({"a":[1,2,3]});
        let_assert!(
            Err(PatchError::ResolveError(_)) =
                replace(&mut doc, "/a/foo".try_into().unwrap(), json!(42))
        );
    }

    #[test]
    fn replace_with_escape_sequences_in_path_should_succeed() {
        let mut doc = json!({"a/b": {"c~d": 1}});

        let_assert!(Ok(()) = replace(&mut doc, "/a~1b/c~0d".try_into().unwrap(), json!(42)));
        check!(doc == json!({"a/b": {"c~d": 42}}));
    }
}
