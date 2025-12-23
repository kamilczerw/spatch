use serde_json::Value;

use crate::{
    patch::{add, error::PatchError, remove},
    path::Spath,
    resolve::resolve_ref,
};

/// The "move" operation removes the value at a specified location and
/// adds it to the target location.
///
/// The operation object MUST contain a "from" member, which is a string
/// containing a JSON Pointer value that references the location in the
/// target document to move the value from.
///
/// The "from" location MUST exist for the operation to be successful.
///
/// For example:
///
/// { "op": "move", "from": "/a/b/c", "path": "/a/b/d" }
///
/// This operation is functionally identical to a "remove" operation on
/// the "from" location, followed immediately by an "add" operation at
/// the target location with the value that was just removed.
///
/// The "from" location MUST NOT be a proper prefix of the "path"
/// location; i.e., a location cannot be moved into one of its children.
pub fn move_op(doc: &mut Value, from: Spath, path: Spath) -> Result<(), PatchError> {
    let value = resolve_ref(doc, &from)?.clone();

    if from.is_parent_of(&path) {
        return Err(PatchError::CannotMoveIntoChild);
    }

    // NOTE: This is not ideal as it clones the entire document, but we need to do it in order to
    // ensure the document is correct if any operation fails. It would be better to resolve the
    // parent of the target path and then do the remove and add operations directly on the
    // document. But for not, this will do.
    let mut doc_cloned = doc.clone();

    remove(&mut doc_cloned, from)?;
    add(&mut doc_cloned, path, value)?;

    *doc = doc_cloned;

    Ok(())
}

#[cfg(test)]
mod tests {
    use assert2::{check, let_assert};
    use serde_json::json;

    use crate::resolve::ResolveError;

    use super::*;

    #[test]
    fn move_of_the_root_should_fail() {
        let mut doc = json!({"a": 1, "b": 2});
        let_assert!(
            Err(PatchError::CannotMoveIntoChild) =
                move_op(&mut doc, "".try_into().unwrap(), "/a".try_into().unwrap())
        );

        check!(doc == json!({"a": 1, "b": 2}));
    }

    #[test]
    fn move_into_child_should_fail() {
        let mut doc = json!({"a": {"b": 2}, "c": 3});
        let_assert!(
            Err(PatchError::CannotMoveIntoChild) = move_op(
                &mut doc,
                "/a".try_into().unwrap(),
                "/a/b".try_into().unwrap()
            )
        );

        check!(doc == json!({"a": {"b": 2}, "c": 3}));
    }

    #[test]
    fn move_existing_field_should_succeed() {
        let mut doc = json!({"a": 1, "b": 2});

        let_assert!(Ok(()) = move_op(&mut doc, "/a".try_into().unwrap(), "/c".try_into().unwrap()));
        check!(doc == json!({"b": 2, "c": 1}));
    }

    #[test]
    fn move_non_existing_field_should_fail() {
        let mut doc = json!({"a": 1, "b": 2});

        let_assert!(
            Err(PatchError::ResolveError(_)) =
                move_op(&mut doc, "/c".try_into().unwrap(), "/d".try_into().unwrap())
        );

        check!(doc == json!({"a": 1, "b": 2}));
    }

    #[test]
    fn move_field_to_nested_object_should_succeed() {
        let mut doc = json!({"a": {"b": 2}, "c": 3});

        let_assert!(
            Ok(()) = move_op(
                &mut doc,
                "/c".try_into().unwrap(),
                "/a/d".try_into().unwrap()
            )
        );
        check!(doc == json!({"a": {"b": 2, "d": 3}}));
    }

    #[test]
    fn move_array_element_should_succeed() {
        let mut doc = json!({"a": [1, 2, 3], "b": []});

        let_assert!(
            Ok(()) = move_op(
                &mut doc,
                "/a/1".try_into().unwrap(),
                "/b/0".try_into().unwrap()
            )
        );
        check!(doc == json!({"a": [1, 3], "b": [2]}));
    }

    #[test]
    fn move_array_element_to_object_field_should_succeed() {
        let mut doc = json!({"a": [1, 2, 3], "b": {}});

        let_assert!(
            Ok(()) = move_op(
                &mut doc,
                "/a/0".try_into().unwrap(),
                "/b/first".try_into().unwrap()
            )
        );
        check!(doc == json!({"a": [2, 3], "b": {"first": 1}}));
    }

    #[test]
    fn move_array_element_to_the_end_of_same_array_should_succeed() {
        let mut doc = json!({"a": [1, 2, 3, 4]});

        let_assert!(
            Ok(()) = move_op(
                &mut doc,
                "/a/1".try_into().unwrap(),
                "/a/3".try_into().unwrap()
            )
        );
        check!(doc == json!({"a": [1, 3, 4, 2]}));
    }

    #[test]
    fn move_array_element_inside_same_array_should_succeed() {
        let mut doc = json!({"a": [1, 2, 3, 4]});

        let_assert!(
            Ok(()) = move_op(
                &mut doc,
                "/a/1".try_into().unwrap(),
                "/a/2".try_into().unwrap()
            )
        );
        check!(doc == json!({"a": [1, 3, 2, 4]}));
    }

    #[test]
    fn move_array_element_out_of_bounds_should_fail() {
        let mut doc = json!({"a": [1, 2, 3]});

        let_assert!(
            Err(PatchError::ResolveError(_)) = move_op(
                &mut doc,
                "/a/5".try_into().unwrap(),
                "/b".try_into().unwrap()
            )
        );
        check!(doc == json!({"a": [1, 2, 3]}));
    }

    #[test]
    fn move_from_non_array_should_fail() {
        let mut doc = json!({"a": {"b": 2}, "c": 3});

        let_assert!(
            Err(PatchError::ResolveError(_)) = move_op(
                &mut doc,
                "/a/b/0".try_into().unwrap(),
                "/c".try_into().unwrap()
            )
        );
        check!(doc == json!({"a": {"b": 2}, "c": 3}));
    }

    #[test]
    fn move_to_non_object_should_fail() {
        let mut doc = json!({"a": {"b": 2}, "c": 3});

        let_assert!(
            Err(PatchError::NotAContainer { parent, actual }) = move_op(
                &mut doc,
                "/c".try_into().unwrap(),
                "/a/b/d".try_into().unwrap()
            )
        );

        check!(parent == "/a/b".try_into().unwrap());
        check!(actual == "number(2)".to_string());

        check!(doc == json!({"a": {"b": 2}, "c": 3}));
    }

    #[test]
    fn move_with_empty_key_should_succeed() {
        let mut doc = json!({"": {"a": 1}, "b": 2});

        let_assert!(Ok(()) = move_op(&mut doc, "/".try_into().unwrap(), "/b".try_into().unwrap()));
        check!(doc == json!({"b": {"a": 1}}));
    }

    #[test]
    fn move_with_special_characters_in_key_should_succeed() {
        let mut doc = json!({"a/b": {"c~d": 1}, "e": 2});

        let_assert!(
            Ok(()) = move_op(
                &mut doc,
                "/a~1b/c~0d".try_into().unwrap(),
                "/e".try_into().unwrap()
            )
        );
        check!(doc == json!({"a/b": {}, "e": 1}));
    }

    #[test]
    fn move_nested_field_should_succeed() {
        let mut doc = json!({"a": {"b": {"c": 1}}, "d": 2});

        let_assert!(
            Ok(()) = move_op(
                &mut doc,
                "/a/b/c".try_into().unwrap(),
                "/d".try_into().unwrap()
            )
        );
        check!(doc == json!({"a": {"b": {}}, "d": 1}));
    }

    #[test]
    fn move_using_filter_path_should_succeed() {
        let mut doc = json!({
            "items": [
                {"id": "item1", "value": "hello"},
                {"id": "item2", "value": "world"}
            ],
            "selected": null
        });

        let_assert!(
            Ok(()) = move_op(
                &mut doc,
                "/items/[id=item2]/value".try_into().unwrap(),
                "/selected".try_into().unwrap()
            )
        );
        check!(
            doc == json!({
                "items": [
                    {"id": "item1", "value": "hello"},
                    {"id": "item2"}
                ],
                "selected": "world"
            })
        );
    }

    #[test]
    fn move_using_filter_path_non_existing_should_fail() {
        let mut doc = json!({
            "items": [
                {"id": "item1", "value": "hello"},
                {"id": "item2", "value": "world"}
            ],
            "selected": null
        });

        let_assert!(
            Err(PatchError::ResolveError(_)) = move_op(
                &mut doc,
                "/items/[id=item3]/value".try_into().unwrap(),
                "/selected".try_into().unwrap()
            )
        );
        check!(
            doc == json!({
                "items": [
                    {"id": "item1", "value": "hello"},
                    {"id": "item2", "value": "world"}
                ],
                "selected": null
            })
        );
    }

    #[test]
    fn move_using_filter_path_into_child_should_fail() {
        let mut doc = json!({
            "items": [
                {"id": "item1", "value": {"text": "hello"}},
                {"id": "item2", "value": {"text": "world"}}
            ]
        });

        let_assert!(
            Err(PatchError::CannotMoveIntoChild) = move_op(
                &mut doc,
                "/items/[id=item1]/value".try_into().unwrap(),
                "/items/[id=item1]/value/text".try_into().unwrap()
            )
        );
        check!(
            doc == json!({
                "items": [
                    {"id": "item1", "value": {"text": "hello"}},
                    {"id": "item2", "value": {"text": "world"}}
                ]
            })
        );
    }

    #[test]
    fn move_entire_array_member_using_filter_should_succeed() {
        let mut doc = json!({
            "items": [
                {"id": "item1", "value": 10},
                {"id": "item2", "value": 20}
            ],
        });

        let_assert!(
            Ok(()) = move_op(
                &mut doc,
                "/items/[id=item2]".try_into().unwrap(),
                "/removed".try_into().unwrap()
            )
        );
        check!(
            doc == json!({
                "items": [
                    {"id": "item1", "value": 10}
                ],
                "removed": {"id": "item2", "value": 20}
            })
        );
    }

    #[test]
    fn move_entire_array_member_using_filter_to_the_same_array_should_fail() {
        let mut doc = json!({
            "items": [
                {"id": "item1", "value": 10},
                {"id": "item2", "value": 20}
            ],
        });

        let_assert!(
            Err(PatchError::MissingFinalToken { path }) = move_op(
                &mut doc,
                "/items/[id=item1]".try_into().unwrap(),
                "/items/[id=item2]".try_into().unwrap()
            )
        );

        check!(path == "/items/[id=item2]".try_into().unwrap());

        check!(
            doc == json!({
                "items": [
                    {"id": "item1", "value": 10},
                    {"id": "item2", "value": 20}
                ],
            })
        );
    }

    #[test]
    fn move_entire_array_member_using_filter_and_index_should_succeed() {
        let mut doc = json!({
            "items": [
                {"id": "item1", "value": 10},
                {"id": "item2", "value": 20},
                {"id": "item3", "value": 30}
            ],
        });

        let_assert!(
            Ok(()) = move_op(
                &mut doc,
                "/items/[id=item2]".try_into().unwrap(),
                "/items/0".try_into().unwrap()
            )
        );
        check!(
            doc == json!({
                "items": [
                    {"id": "item2", "value": 20},
                    {"id": "item1", "value": 10},
                    {"id": "item3", "value": 30}
                ],
            })
        );
    }

    #[test]
    fn move_non_existing_to_child_should_fail() {
        let mut doc = json!({"a": 1, "b": 2});
        let_assert!(
            Err(PatchError::ResolveError(ResolveError::NotFound)) = move_op(
                &mut doc,
                "/c".try_into().unwrap(),
                "/c/d".try_into().unwrap()
            )
        );

        check!(doc == json!({"a": 1, "b": 2}));
    }

    #[test]
    fn move_to_the_same_location_should_succeed() {
        let mut doc = json!({"a": 1, "b": 2});
        let_assert!(Ok(()) = move_op(&mut doc, "/a".try_into().unwrap(), "/a".try_into().unwrap()));

        check!(doc == json!({"a": 1, "b": 2}));
    }
}
