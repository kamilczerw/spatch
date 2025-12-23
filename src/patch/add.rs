use serde_json::Value;

use crate::{
    patch::error::PatchError,
    path::Spath,
    resolve::{resolve_mut, value_type_desc},
};

/// The "add" operation performs one of the following functions,
/// depending upon what the target location references:
///
/// o  If the target location specifies an array index, a new value is
///     inserted into the array at the specified index.
///
/// o  If the target location specifies an object member that does not
///     already exist, a new member is added to the object.
///
/// o  If the target location specifies an object member that does exist,
///    that member's value is replaced.
///
/// The operation object MUST contain a "value" member whose content
/// specifies the value to be added.
///
/// For example:
///
/// { "op": "add", "path": "/a/b/c", "value": [ "foo", "bar" ] }
///
/// When the operation is applied, the target location MUST reference one
/// of:
///
/// o  The root of the target document - whereupon the specified value
///    becomes the entire content of the target document.
///
/// o  A member to add to an existing object - whereupon the supplied
///    value is added to that object at the indicated location.  If the
///    member already exists, it is replaced by the specified value.
///
/// o  An element to add to an existing array - whereupon the supplied
///    value is added to the array at the indicated location.  Any
///    elements at or above the specified index are shifted one position
///    to the right.  The specified index MUST NOT be greater than the
///    number of elements in the array.  If the "-" character is used to
///    index the end of the array (see [RFC6901]), this has the effect of
///    appending the value to the array.
///
/// Because this operation is designed to add to existing objects and
/// arrays, its target location will often not exist.  Although the
/// pointer's error handling algorithm will thus be invoked, this
/// specification defines the error handling behavior for "add" pointers
/// to ignore that error and add the value as specified.
///
/// However, the object itself or an array containing it does need to
/// exist, and it remains an error for that not to be the case.  For
/// example, an "add" with a target location of "/a/b" starting with this
/// document:
///
/// { "a": { "foo": 1 } }
///
/// is not an error, because "a" exists, and "b" will be added to its
/// value.  It is an error in this document:
///
/// { "q": { "bar": 2 } }
///
/// because "a" does not exist.
pub fn add(doc: &mut Value, path: Spath, value: Value) -> Result<(), PatchError> {
    if path.is_empty() {
        *doc = value;
        return Ok(());
    }

    let parent = path.parent().ok_or(PatchError::missing_parent(&path))?;

    let target = resolve_mut(doc, &parent)?;
    let field = path.field().ok_or(PatchError::missing_final_token(&path))?;

    match target {
        Value::Object(obj) => {
            obj.insert(field, value);
        }
        Value::Array(arr) => {
            if field == "-" {
                arr.push(value);
            } else {
                let index: usize = field
                    .parse()
                    .map_err(|_e| PatchError::invalid_array_index_token(&path, &field))?;

                // Spec defines that index must not be greater than the number of elements
                if index > arr.len() {
                    return Err(PatchError::index_out_of_bounds(&path, index, arr.len()));
                }
                // serde implements insert with shifting elements to the right
                // which is the desired behavior according to the spec
                arr.insert(index, value);
            }
        }
        val => {
            return Err(PatchError::not_a_container(&parent, &value_type_desc(val)));
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use assert2::{check, let_assert};
    use serde_json::{from_str, json};

    use crate::resolve::ResolveError;

    use super::*;

    #[test]
    fn add_an_object_at_root() {
        let mut doc: Value = from_str(r#"{"a":1}"#).unwrap();

        add(&mut doc, "".try_into().unwrap(), json!({"foo": "bar"})).unwrap();

        check!(doc == json!({"foo": "bar"}));
    }

    #[test]
    fn add_an_object_at_member_with_empty_name() {
        let mut doc = json!({"a": 1});

        add(&mut doc, "/".try_into().unwrap(), json!({"foo": "bar"})).unwrap();

        check!(doc == json!({"a": 1, "": {"foo": "bar"}}));
    }

    #[test]
    fn add_an_object_at_a_path() {
        let mut doc: Value = from_str(r#"{"a":1}"#).unwrap();

        add(&mut doc, "/a".try_into().unwrap(), json!({"foo": "bar"})).unwrap();

        check!(doc == json!({"a": {"foo": "bar"}}));
    }

    #[test]
    fn add_an_object_at_a_missing_path_with_existing_parent() {
        let mut doc: Value = json!({"a": {"foo": 1}});

        add(&mut doc, "/a/b".try_into().unwrap(), json!({"foo": "bar"})).unwrap();

        check!(doc == json!({"a": {"b": {"foo": "bar"},"foo": 1}}));
    }

    #[test]
    fn add_an_object_at_an_existing_path_with_existing_parent() {
        let mut doc: Value = json!({"a": {"foo": 1, "bar": 123}});

        add(
            &mut doc,
            "/a/bar".try_into().unwrap(),
            json!({"foo": "bar"}),
        )
        .unwrap();

        check!(doc == json!({"a": {"foo": 1, "bar": {"foo": "bar"}}}));
    }

    #[test]
    fn add_an_object_at_a_missing_path_with_empty_field() {
        let mut doc: Value = json!({"a": {"foo": 1}});

        add(&mut doc, "/a/".try_into().unwrap(), json!({"foo": "bar"})).unwrap();

        check!(doc == json!({"a": {"foo": 1, "": {"foo": "bar"}}}));
    }

    #[test]
    fn add_an_object_to_non_existent_path_should_fail() {
        let mut doc: Value = from_str(r#"{"a":1}"#).unwrap();

        let result = add(&mut doc, "/b/c".try_into().unwrap(), json!({"foo": "bar"}));

        let_assert!(Err(PatchError::ResolveError(e)) = result);
        check!(e.to_string() == "Field or item not found");
    }

    #[test]
    fn add_to_array_at_specific_index() {
        let mut doc: Value = from_str(r#"{"a":[1,2,3]}"#).unwrap();

        add(&mut doc, "/a/1".try_into().unwrap(), json!(99)).unwrap();

        check!(doc == json!({"a":[1,99,2,3]}));
    }

    #[test]
    fn add_to_array_at_end() {
        let mut doc: Value = from_str(r#"{"a":[1,2,3]}"#).unwrap();

        add(&mut doc, "/a/3".try_into().unwrap(), json!(99)).unwrap();

        check!(doc == json!({"a":[1,2,3,99]}));
    }

    #[test]
    fn add_to_array_using_append() {
        let mut doc: Value = from_str(r#"{"a":[1,2,3]}"#).unwrap();

        add(&mut doc, "/a/-".try_into().unwrap(), json!(99)).unwrap();

        check!(doc == json!({"a":[1,2,3,99]}));
    }

    #[test]
    fn add_to_array_at_out_of_bounds_index_should_fail() {
        let mut doc: Value = from_str(r#"{"a":[1,2,3]}"#).unwrap();

        let result = add(&mut doc, "/a/5".try_into().unwrap(), json!(99));

        let_assert!(Err(PatchError::ArrayIndexOutOfBounds { path, index, len }) = result);

        check!(path == "/a/5".try_into().unwrap());
        check!(index == 5);
        check!(len == 3);
    }

    // -----------------------------
    // Root semantics: "" vs "/"
    // -----------------------------

    #[test]
    fn add_with_empty_path_replaces_document() {
        let mut doc: Value = json!({"a": 1});

        add(&mut doc, "".try_into().unwrap(), json!(null)).unwrap();

        check!(doc == json!(null));
    }

    #[test]
    fn add_with_slash_path_adds_empty_key_on_object_root() {
        let mut doc: Value = json!({"a": 1});

        add(&mut doc, "/".try_into().unwrap(), json!({"foo": "bar"})).unwrap();

        // JSON Pointer "/" means token "" at root object
        check!(doc == json!({"a": 1, "": {"foo": "bar"}}));
    }

    // -----------------------------
    // Parent must exist and be a container
    // -----------------------------

    #[test]
    fn add_when_parent_is_scalar_should_fail() {
        let mut doc: Value = json!({"a": 1});

        // parent "/a" exists but is not object/array
        let result = add(&mut doc, "/a/b".try_into().unwrap(), json!(123));

        let_assert!(Err(PatchError::NotAContainer { parent, actual }) = result);

        check!(parent == "/a".try_into().unwrap());
        check!(actual == "number(1)");
    }

    #[test]
    fn add_to_root_when_doc_is_scalar_should_fail_for_non_empty_path() {
        let mut doc: Value = json!(1);

        let result = add(&mut doc, "/a".try_into().unwrap(), json!(2));

        check!(result.is_err());
    }

    #[test]
    fn add_with_sem_path_when_parent_is_scalar_should_fail() {
        let mut doc: Value = json!([{"id": "foo", "value": 1}, {"id": "bar", "value": 2}]);

        // parent "/a" exists but is not object/array
        let result = add(
            &mut doc,
            "/[id=foo]/value/non-existing".try_into().unwrap(),
            json!(123),
        );

        let_assert!(Err(PatchError::NotAContainer { parent, actual }) = result);

        check!(parent == "/[id=foo]/value".try_into().unwrap());
        check!(actual == "number(1)");
    }

    // -----------------------------
    // Object edge cases
    // -----------------------------

    #[test]
    fn add_object_key_named_dash_is_normal_key() {
        let mut doc: Value = json!({});

        add(&mut doc, "/-".try_into().unwrap(), json!(123)).unwrap();

        check!(doc == json!({"-": 123}));
    }

    #[test]
    fn add_object_key_with_json_pointer_escaped_slash() {
        // key is literally "a/b"
        let mut doc: Value = json!({});

        add(&mut doc, "/a~1b".try_into().unwrap(), json!(1)).unwrap();

        check!(doc == json!({"a/b": 1}));
    }

    #[test]
    fn add_object_key_with_json_pointer_escaped_tilde() {
        // key is literally "a~b"
        let mut doc: Value = json!({});

        add(&mut doc, "/a~0b".try_into().unwrap(), json!(1)).unwrap();

        check!(doc == json!({"a~b": 1}));
    }

    // If your Spath doesn't decode ~0/~1, the above two tests will fail,
    // which is good: it tells you escaping isn't being handled.

    // -----------------------------
    // Array insertion edge cases
    // -----------------------------

    #[test]
    fn add_to_array_at_index_zero_inserts_at_front() {
        let mut doc: Value = json!({"a":[1,2,3]});

        add(&mut doc, "/a/0".try_into().unwrap(), json!(99)).unwrap();

        check!(doc == json!({"a":[99,1,2,3]}));
    }

    #[test]
    fn add_to_empty_array_at_index_zero_is_ok() {
        let mut doc: Value = json!({"a":[]});

        add(&mut doc, "/a/0".try_into().unwrap(), json!(99)).unwrap();

        check!(doc == json!({"a":[99]}));
    }

    #[test]
    fn add_to_empty_array_using_append_is_ok() {
        let mut doc: Value = json!({"a":[]});

        add(&mut doc, "/a/-".try_into().unwrap(), json!(99)).unwrap();

        check!(doc == json!({"a":[99]}));
    }

    #[test]
    fn add_to_array_with_negative_index_should_fail() {
        let mut doc: Value = json!({"a":[1,2,3]});

        let result = add(&mut doc, "/a/-1".try_into().unwrap(), json!(99));

        let_assert!(Err(PatchError::InvalidArrayIndexToken { path, token }) = result);

        check!(path == "/a/-1".try_into().unwrap());
        check!(token == "-1");
    }

    #[test]
    fn add_to_array_with_non_numeric_index_should_fail() {
        let mut doc: Value = json!({"a":[1,2,3]});

        let result = add(&mut doc, "/a/notanumber".try_into().unwrap(), json!(99));

        let_assert!(Err(PatchError::InvalidArrayIndexToken { path, token }) = result);

        check!(path == "/a/notanumber".try_into().unwrap());
        check!(token == "notanumber");
    }

    #[test]
    fn add_to_array_with_float_index_should_fail() {
        let mut doc: Value = json!({"a":[1,2,3]});

        let result = add(&mut doc, "/a/1.0".try_into().unwrap(), json!(99));

        let_assert!(Err(PatchError::InvalidArrayIndexToken { path, token }) = result);

        check!(path == "/a/1.0".try_into().unwrap());
        check!(token == "1.0");
    }

    #[test]
    fn add_to_array_using_dash_not_at_last_segment_should_fail() {
        let mut doc: Value = json!({"a":[{"b": 1}]});

        // "-” only makes sense as the final token of the path.
        // Here parent resolution will likely fail trying to resolve index "-"
        let result = add(&mut doc, "/a/-/b".try_into().unwrap(), json!(99));

        check!(result.is_err());
    }

    #[test]
    fn add_to_array_parent_is_object_element() {
        let mut doc: Value = json!({"a":[{"b": 1}]});

        add(&mut doc, "/a/0/c".try_into().unwrap(), json!(2)).unwrap();

        check!(doc == json!({"a":[{"b": 1, "c": 2}]}));
    }

    #[test]
    fn add_to_array_element_parent_is_scalar_should_fail() {
        let mut doc: Value = json!({"a":[1]});

        // parent "/a/0" exists but is scalar
        let result = add(&mut doc, "/a/0/b".try_into().unwrap(), json!(2));

        let_assert!(Err(PatchError::NotAContainer { parent, actual }) = result);

        check!(parent == "/a/0".try_into().unwrap());
        check!(actual == "number(1)");
    }

    // -----------------------------
    // Path parsing / empty segments
    // -----------------------------

    #[test]
    fn add_empty_field_key_under_object() {
        let mut doc: Value = json!({"a": {}});

        add(&mut doc, "/a/".try_into().unwrap(), json!(1)).unwrap();

        check!(doc == json!({"a": {"": 1}}));
    }

    #[test]
    fn add_double_slash_creates_empty_key_then_key() {
        // Path "/a//b" => tokens ["a", "", "b"]
        let mut doc: Value = json!({"a": { "": {} }});

        add(&mut doc, "/a//b".try_into().unwrap(), json!(1)).unwrap();

        check!(doc == json!({"a": { "": { "b": 1 }}}));
    }

    // -----------------------------
    // Semantic path tests
    // -----------------------------
    #[test]
    fn add_using_semantic_path_should_succeed() {
        let mut doc: Value = json!({
            "items": [
                { "id": "foo", "value": 1 },
                { "id": "bar", "value": 2 }
            ]
        });

        add(
            &mut doc,
            "/items/[id=foo]/new_field".try_into().unwrap(),
            json!(99),
        )
        .unwrap();

        check!(
            doc == json!({
                "items": [
                    { "id": "foo", "value": 1, "new_field": 99 },
                    { "id": "bar", "value": 2 }
                ]
            })
        );
    }

    #[test]
    fn add_to_array_using_semantic_path_with_non_existing_array_member_should_fail() {
        let mut doc: Value = json!({
            "items": [
                { "id": "foo", "value": 1 },
                { "id": "bar", "value": 2 }
            ]
        });

        let result = add(
            &mut doc,
            "/items/[id=baz]/new_field".try_into().unwrap(),
            json!(99),
        );

        let_assert!(Err(PatchError::ResolveError(ResolveError::NotFound)) = result);
    }

    #[test]
    fn add_using_semantic_path_with_non_existing_array_member_should_fail() {
        let mut doc: Value = json!({
            "items": [
                { "id": "foo", "value": 1 },
                { "id": "bar", "value": 2 }
            ]
        });

        let result = add(&mut doc, "/items/[id=baz]".try_into().unwrap(), json!(99));

        let_assert!(Err(PatchError::MissingFinalToken { path }) = result);

        check!(path == "/items/[id=baz]".try_into().unwrap());
    }

    #[test]
    fn add_using_semantic_path_with_multiple_filters_should_succeed() {
        let mut doc: Value = json!({
            "items": [
                { "id": "foo", "value": 1 },
                { "id": "bar", "value": 2 }
            ]
        });

        let result = add(
            &mut doc,
            "/items/[id=foo, value=1]/baz".try_into().unwrap(),
            json!(99),
        );

        let_assert!(Ok(()) = result);
        check!(
            doc == json!({
                "items": [
                    { "id": "foo", "value": 1, "baz": 99 },
                    { "id": "bar", "value": 2 }
                ]
            })
        );
    }

    #[test]
    fn add_using_semantic_path_with_multiple_filters_should_fail() {
        let mut doc: Value = json!({
            "items": [
                { "id": "foo", "value": 1 },
                { "id": "bar", "value": 2 }
            ]
        });

        let result = add(
            &mut doc,
            "/items/[id=foo, value=2]/baz".try_into().unwrap(),
            json!(99),
        );

        let_assert!(Err(PatchError::ResolveError(ResolveError::NotFound)) = result);
    }
}
