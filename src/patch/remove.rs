use serde_json::Value;

use crate::{
    patch::error::PatchError,
    path::{Segment, Spath},
    resolve::{ResolveError, resolve_mut, value_type_desc},
};

/// The "remove" operation removes the value at the target location.
///
/// The target location MUST exist for the operation to be successful.
///
/// For example:
///
/// { "op": "remove", "path": "/a/b/c" }
///
/// If removing an element from an array, any elements above the
/// specified index are shifted one position to the left.
pub fn remove(doc: &mut Value, path: Spath) -> Result<(), PatchError> {
    if path.is_empty() {
        return Err(PatchError::CannotRemoveRoot);
    }

    let parent = path.parent().ok_or(PatchError::missing_parent(&path))?;

    let target = resolve_mut(doc, &parent)?;

    match target {
        Value::Object(map) => {
            let field = path.field().ok_or(PatchError::missing_final_token(&path))?;
            map.remove(&field)
                .ok_or(PatchError::target_not_found(&path))?;
        }
        Value::Array(arr) => {
            let segment = path.last_segment().ok_or(PatchError::CannotRemoveRoot)?;
            let index: usize = match segment {
                Segment::Field(field) => field
                    .parse()
                    .map_err(|_| PatchError::invalid_array_index_token(&path, field))?,
                Segment::Filter(filters) => find_array_index(arr, filters)
                    .ok_or(PatchError::ResolveError(ResolveError::NotFound))?,
            };

            if index >= arr.len() {
                return Err(PatchError::index_out_of_bounds(&path, index, arr.len()));
            }
            arr.remove(index);
        }
        val => {
            return Err(PatchError::not_a_container(&parent, &value_type_desc(val)));
        }
    }
    Ok(())
}

fn find_array_index(arr: &[Value], filters: &[(String, String)]) -> Option<usize> {
    for (index, item) in arr.iter().enumerate() {
        if filters.iter().all(|(k, v)| {
            item.get(k)
                .and_then(|val| val.as_str())
                .is_some_and(|val_str| val_str == v)
        }) {
            return Some(index);
        }
    }
    None
}

#[cfg(test)]
mod tests {

    use assert2::{check, let_assert};
    use serde_json::json;

    use crate::resolve::ResolveError;

    use super::*;

    #[test]
    fn remove_empty_path_should_fail() {
        let mut doc = json!({"a": 1, "b": 2});

        let_assert!(Err(PatchError::CannotRemoveRoot) = remove(&mut doc, "".try_into().unwrap()));
    }

    #[test]
    fn remove_root_path_should_remove_a_document_at_empty_key() {
        let mut doc = json!({"a": 1, "b": 2, "": 3});

        let_assert!(Ok(()) = remove(&mut doc, "/".try_into().unwrap()));

        check!(doc == json!({"a": 1, "b": 2}));
    }

    #[test]
    fn remove_existing_field_should_succeed() {
        let mut doc = json!({"a": 1, "b": 2});

        let_assert!(Ok(()) = remove(&mut doc, "/a".try_into().unwrap()));

        check!(doc == json!({"b": 2}));
    }

    #[test]
    fn remove_non_existing_field_should_fail() {
        let mut doc = json!({"a": 1, "b": 2});

        let_assert!(
            Err(PatchError::TargetNotFound { path }) = remove(&mut doc, "/c".try_into().unwrap())
        );

        check!(path == "/c".try_into().unwrap());

        check!(doc == json!({"a": 1, "b": 2}));
    }

    #[test]
    fn remove_field_from_non_object_should_fail() {
        let mut doc = json!({"a": [1, 2, 3]});

        let_assert!(
            Err(PatchError::InvalidArrayIndexToken { path, token }) =
                remove(&mut doc, "/a/b".try_into().unwrap())
        );

        check!(path == "/a/b".try_into().unwrap());
        check!(token == "b");

        check!(doc == json!({"a": [1, 2, 3]}));
    }

    #[test]
    fn remove_field_from_nested_object_should_succeed() {
        let mut doc = json!({"a": {"b": {"c": 3, "d": 4}}, "e": 5});

        let_assert!(Ok(()) = remove(&mut doc, "/a/b/c".try_into().unwrap()));

        check!(doc == json!({"a": {"b": {"d": 4}}, "e": 5}));
    }

    #[test]
    fn remove_field_with_filter_should_succeed() {
        let mut doc = json!({
            "items": [
                {"id": "item1", "value": 10},
                {"id": "item2", "value": 20}
            ]
        });

        let_assert!(Ok(()) = remove(&mut doc, "/items/[id=item1]/value".try_into().unwrap()));

        check!(
            doc == json!({
                "items": [
                    {"id": "item1"},
                    {"id": "item2", "value": 20}
                ]
            })
        );
    }

    #[test]
    fn remove_field_with_non_matching_filter_should_fail() {
        let mut doc = json!({
            "items": [
                {"id": "item1", "value": 10},
                {"id": "item2", "value": 20}
            ]
        });

        let_assert!(
            Err(PatchError::ResolveError(ResolveError::NotFound)) =
                remove(&mut doc, "/items/[id=item3]/value".try_into().unwrap())
        );

        check!(
            doc == json!({
                "items": [
                    {"id": "item1", "value": 10},
                    {"id": "item2", "value": 20}
                ]
            })
        );
    }

    #[test]
    fn remove_from_empty_document_should_fail() {
        let mut doc = json!({});

        let_assert!(
            Err(PatchError::TargetNotFound { path }) = remove(&mut doc, "/a".try_into().unwrap())
        );

        check!(path == "/a".try_into().unwrap());

        check!(doc == json!({}));
    }

    #[test]
    fn remove_from_array_should_succeed() {
        let mut doc = json!([1, 2, 3]);

        let_assert!(Ok(()) = remove(&mut doc, "/0".try_into().unwrap()));

        check!(doc == json!([2, 3]));
    }

    #[test]
    fn remove_from_array_out_of_bounds_should_fail() {
        let mut doc = json!([1, 2, 3]);

        let_assert!(
            Err(PatchError::ArrayIndexOutOfBounds { path, index, len }) =
                remove(&mut doc, "/3".try_into().unwrap())
        );

        check!(path == "/3".try_into().unwrap());
        check!(index == 3);
        check!(len == 3);

        check!(doc == json!([1, 2, 3]));
    }

    #[test]
    fn remove_from_non_array_should_fail() {
        let mut doc = json!({"a": 1, "b": 2});

        let_assert!(
            Err(PatchError::NotAContainer { parent, actual }) =
                remove(&mut doc, "/a/0".try_into().unwrap())
        );

        check!(parent == "/a".try_into().unwrap());
        check!(actual == "number(1)");

        check!(doc == json!({"a": 1, "b": 2}));
    }

    #[test]
    fn remove_with_empty_key_and_nested_path_should_succeed() {
        let mut doc = json!({"a": {"": {"b": 1}}, "b": 2});

        let_assert!(Ok(()) = remove(&mut doc, "/a//b".try_into().unwrap()));

        check!(doc == json!({"a": {"": {}}, "b": 2}));
    }

    #[test]
    fn remove_with_special_characters_in_key_should_succeed() {
        let mut doc = json!({"a/b": {"c~d": 1}, "e": 2});

        let_assert!(Ok(()) = remove(&mut doc, "/a~1b/c~0d".try_into().unwrap()));

        check!(doc == json!({"a/b": {}, "e": 2}));
    }

    #[test]
    fn remove_with_semantic_filter_should_succeed() {
        let mut doc = json!({
            "items": [
                {"id": "item1", "value": 10},
                {"id": "item2", "value": 20}
            ]
        });

        let_assert!(Ok(()) = remove(&mut doc, "/items/[id=item2]".try_into().unwrap()));

        check!(
            doc == json!({
                "items": [
                    {"id": "item1", "value": 10}
                ]
            })
        );
    }

    #[test]
    fn remove_nested_with_semantic_filter_should_succeed() {
        let mut doc = json!({
            "items": [
                {"id": "item1", "value": 10},
                {"id": "item2", "value": 20}
            ]
        });

        let_assert!(Ok(()) = remove(&mut doc, "/items/[id=item2]/value".try_into().unwrap()));
        check!(
            doc == json!({
                "items": [
                    {"id": "item1", "value": 10},
                    {"id": "item2"}
                ]
            })
        );
    }

    #[test]
    fn remove_non_existing_with_semantic_filter_should_fail() {
        let mut doc = json!({
            "items": [
                {"id": "item1", "value": 10},
                {"id": "item2", "value": 20}
            ]
        });

        let_assert!(
            Err(PatchError::ResolveError(ResolveError::NotFound)) =
                remove(&mut doc, "/items/[id=item3]".try_into().unwrap())
        );

        check!(
            doc == json!({
                "items": [
                    {"id": "item1", "value": 10},
                    {"id": "item2", "value": 20}
                ]
            })
        );
    }

    #[test]
    fn remove_with_multiple_filters_should_succeed() {
        let mut doc = json!({
            "items": [
                {"id": "item1", "type": "A", "value": 10},
                {"id": "item2", "type": "B", "value": 20},
                {"id": "item3", "type": "A", "value": 30}
            ]
        });

        let_assert!(
            Ok(()) = remove(
                &mut doc,
                "/items/[type=A, id=item3]/value".try_into().unwrap()
            )
        );

        check!(
            doc == json!({
                "items": [
                    {"id": "item1", "type": "A", "value": 10},
                    {"id": "item2", "type": "B", "value": 20},
                    {"id": "item3", "type": "A"}
                ]
            })
        );
    }
}
