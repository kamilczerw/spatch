use std::collections::{HashMap, HashSet};

use serde_json::Value;

use crate::path::Spath;

use super::Patch;

/// Name of the property in the JSON Schema that indicates the index key for arrays.
/// It's used to identify unique items in an array for diffing purposes.
const HASH_KEY_PROP_NAME: &str = "indexKey";

pub(super) fn diff_recursive(
    left: &serde_json::Value,
    right: &serde_json::Value,
    schema: Option<&serde_json::Value>,
    path_pos: &Spath,
    patch_ops: &Patch,
) -> Patch {
    match (left, right) {
        (Value::Object(left_map), Value::Object(right_map)) => {
            diff_object(left_map, right_map, schema, path_pos, patch_ops)
        }
        (Value::Array(left_array), Value::Array(right_array)) => {
            diff_array(left_array, right_array, schema, path_pos, patch_ops)
        }
        (left, right) if left == right => Patch::default(), // Values are equal, no diff needed
        (_, right) => {
            let patch = super::PatchOp::replace(path_pos.clone(), right.clone());
            // patch_ops.push(patch.clone());

            Patch::new(vec![patch])
        }
    }
}

fn diff_object(
    left_map: &serde_json::Map<String, Value>,
    right_map: &serde_json::Map<String, Value>,
    schema: Option<&serde_json::Value>,
    path_pointer: &Spath,
    patch_ops: &Patch,
) -> Patch {
    let inner_patch = right_map
        .iter()
        .map(|(key, right_value)| {
            let sub_schema =
                schema.and_then(|s| s.get("properties").and_then(|props| props.get(key)));
            match left_map.get(key) {
                // If the key exists in both maps, recurse into the values
                Some(left_value) => {
                    let child_path = path_pointer.push(crate::path::Segment::Field(key.clone()));
                    diff_recursive(left_value, right_value, sub_schema, &child_path, patch_ops)
                }
                // Otherwise, it's an addition
                None => {
                    let child_path = path_pointer.push(crate::path::Segment::Field(key.clone()));
                    let patch_op = super::PatchOp::add(child_path.clone(), right_value.clone());

                    Patch::new(vec![patch_op])
                }
            }
        })
        .fold(Patch::default(), |acc, p| acc + p);

    let mut removals = vec![];
    for key in left_map.keys() {
        // If the key is missing in the right map, it's a removal
        if !right_map.contains_key(key) {
            let child_path = path_pointer.push(crate::path::Segment::Field(key.clone()));
            let child_op = super::PatchOp::remove(child_path.clone());
            removals.push(child_op);
        }
    }

    let replace_patch = {
        let patch_op =
            super::PatchOp::replace(path_pointer.clone(), Value::Object(right_map.clone()));
        Patch::new(vec![patch_op])
    };

    let computed_patch = inner_patch + Patch::new(removals);

    let inner_patch_size_bytes = serde_json::to_vec(&computed_patch).unwrap().len();
    let replace_patch_size_bytes = serde_json::to_vec(&replace_patch).unwrap().len();

    if replace_patch_size_bytes < inner_patch_size_bytes {
        replace_patch
    } else {
        computed_patch
    }
}

fn diff_array(
    left: &[Value],
    right: &[Value],
    schema: Option<&Value>,
    path_pointer: &Spath,
    patch_ops: &Patch,
) -> Patch {
    // TODO: emit warning if the schema is missing an index key when the schema is provided
    let index_key = schema.and_then(|s| {
        s.get(HASH_KEY_PROP_NAME)
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
    });
    match (schema, index_key) {
        // If the schema specifies an index key, use keyed diffing
        (Some(schema), Some(ref key)) => {
            diff_array_keyed(left, right, key, schema, path_pointer, patch_ops)
        }
        // Otherwise, use index based diffing
        (_, _) => diff_array_indexed(left, right, schema, path_pointer, patch_ops),
    }
}

fn diff_array_keyed(
    left: &[Value],
    right: &[Value],
    index_key: &str,
    schema: &Value,
    path_pointer: &Spath,
    patch_ops: &Patch,
) -> Patch {
    // Build maps: key -> element
    let map_left = build_key_map(left, index_key);
    let map_right = build_key_map(right, index_key);

    let keys_a: HashSet<_> = map_left.keys().cloned().collect();
    let keys_b: HashSet<_> = map_right.keys().cloned().collect();

    // Removed elements
    let removed = keys_a
        .difference(&keys_b)
        .map(|key| {
            let child_path = path_pointer.push_filter(index_key, key);
            Patch::new_with_op(super::PatchOp::remove(child_path.clone()))
        })
        .fold(Patch::default(), |acc, p| acc + p);

    // Added elements
    let added = keys_b
        .difference(&keys_a)
        .map(|key| {
            let child_path = path_pointer.push_filter(index_key, key);
            let val = &map_right[key];

            Patch::new_with_op(super::PatchOp::add(child_path.clone(), val.clone()))
        })
        .fold(Patch::default(), |acc, p| acc + p);

    let item_schema = schema.get("items");

    // Modified elements (same key in both)
    let modified = keys_a
        .intersection(&keys_b)
        .map(|key| {
            let child_path = path_pointer.push_filter(index_key, key);
            let value_left = &map_left[key];
            let value_right = &map_right[key];

            diff_recursive(value_left, value_right, item_schema, &child_path, patch_ops)
        })
        .fold(Patch::default(), |acc, p| acc + p);
    removed + added + modified
}

fn build_key_map(arr: &[Value], index_key: &str) -> HashMap<String, Value> {
    let mut map = HashMap::new();
    for item in arr {
        if let Value::Object(obj) = item
            && let Some(Value::String(k)) = obj.get(index_key)
        {
            // Last one wins if there are duplicates; you might want to error instead.
            map.insert(k.clone(), item.clone());
        }
    }
    map
}

fn diff_array_indexed(
    left_array: &[Value],
    right_array: &[Value],
    schema: Option<&Value>,
    path_pointer: &Spath,
    patch_ops: &Patch,
) -> Patch {
    let len_left = left_array.len();
    let len_right = right_array.len();

    let item_schema = schema.and_then(|s| s.get("items"));

    // --------
    // Fast paths (deterministic, human-friendly)
    // --------

    // 1) Pure truncate from the end: right is a prefix of left
    // left:  [a,b,c]
    // right: [a,b]
    if len_right <= len_left && left_array[..len_right] == *right_array {
        // remove from the end, descending indexes
        let mut patch = Patch::default();
        for i in (len_right..len_left).rev() {
            let child_path = path_pointer.push(crate::path::Segment::Field(i.to_string()));
            patch = patch + Patch::new_with_op(super::PatchOp::remove(child_path));
        }
        return patch;
    }

    // 2) Pure remove-from-front: right is a suffix of left
    // left:  [a,b,c]
    // right: [b,c]
    if len_right <= len_left && left_array[len_left - len_right..] == *right_array {
        // remove index 0 repeatedly
        let mut patch = Patch::default();
        for _ in 0..(len_left - len_right) {
            let child_path = path_pointer.push(crate::path::Segment::Field("0".to_owned()));
            patch = patch + Patch::new_with_op(super::PatchOp::remove(child_path));
        }
        return patch;
    }

    // 3) Pure append: left is a prefix of right
    // left:  [a,b]
    // right: [a,b,c]
    if len_left <= len_right && right_array[..len_left] == *left_array {
        let mut patch = Patch::default();
        for el in &right_array[len_left..] {
            let child_path = path_pointer.push(crate::path::Segment::Field("-".to_owned()));
            patch = patch + Patch::new_with_op(super::PatchOp::add(child_path, el.clone()));
        }
        return patch;
    }

    // 4) Pure add-to-front: left is a suffix of right
    // left:  [b,c]
    // right: [a,b,c]
    //
    // JSON Patch has no "insert at front" primitive; it’s still `add /arr/0`.
    if len_left <= len_right && right_array[len_right - len_left..] == *left_array {
        let mut patch = Patch::default();
        // add to front in increasing order so final order matches `right`
        for el in right_array[..(len_right - len_left)].iter().rev() {
            // inserting multiple at index 0: do it in reverse so final order is correct
            let child_path = path_pointer.push(crate::path::Segment::Field("0".to_owned()));
            patch = patch + Patch::new_with_op(super::PatchOp::add(child_path, el.clone()));
        }
        return patch;
    }

    // --------
    // Fallback:
    // --------

    let min_len = len_left.min(len_right);

    let recursed = (0..min_len)
        .map(|i| {
            let child_path = path_pointer.push(crate::path::Segment::Field(i.to_string()));
            diff_recursive(
                &left_array[i],
                &right_array[i],
                item_schema,
                &child_path,
                patch_ops,
            )
        })
        .fold(Patch::default(), |acc, p| acc + p);

    // Extra elements in left_array (removals)
    // IMPORTANT: remove from end to avoid index shifting
    let removals = (min_len..len_left)
        .rev()
        .map(|i| {
            let child_path = path_pointer.push(crate::path::Segment::Field(i.to_string()));
            Patch::new_with_op(super::PatchOp::remove(child_path))
        })
        .fold(Patch::default(), |acc, p| acc + p);

    // Extra elements in right_array (additions)
    let additions = right_array[min_len..]
        .iter()
        .map(|element| {
            let child_path = path_pointer.push(crate::path::Segment::Field("-".to_owned()));
            Patch::new_with_op(super::PatchOp::add(child_path, element.clone()))
        })
        .fold(Patch::default(), |acc, p| acc + p);

    recursed + removals + additions
}

#[cfg(test)]
mod tests {
    use assert2::check;

    use crate::diff::PatchOp;
    use crate::diff::test_util::SIMPLE_SCHEMA;
    use crate::diff::test_util::json_patch_tests;

    use super::*;

    fn path(raw: &str) -> Spath {
        raw.try_into().unwrap()
    }

    #[test]
    fn test_diff_recursive_equal_values() {
        let left = serde_json::json!("foo");
        let right = serde_json::json!("foo");

        let patch_ops = diff_recursive(&left, &right, None, &Spath::default(), &Patch::default());

        // No patch operations should be generated for equal values
        check!(patch_ops == Patch::default());
    }

    #[test]
    fn test_diff_recursive_non_equal_values() {
        let left = serde_json::json!("foo");
        let right = serde_json::json!("bar");

        let patch_ops = diff_recursive(&left, &right, None, &Spath::default(), &Patch::default());

        let expected_patch = Patch::new(vec![PatchOp::replace(Spath::default(), right.clone())]);
        check!(patch_ops == expected_patch);
    }

    #[test]
    fn test_diff_recursive_with_left_string_and_right_object() {
        let left = serde_json::json!("foo");
        let right = serde_json::json!({"baz": 42});

        let patch_ops = diff_recursive(&left, &right, None, &Spath::default(), &Patch::default());

        let expected_patch = Patch::new(vec![PatchOp::replace(Spath::default(), right.clone())]);
        check!(patch_ops == expected_patch);
    }

    #[test]
    fn test_diff_recursive_with_both_objects() {
        let left = serde_json::json!({"foo": 43});
        let right = serde_json::json!({"foo": 42});

        let patch_ops = diff_recursive(&left, &right, None, &Spath::default(), &Patch::default());

        let expected_patch = Patch::new(vec![PatchOp::replace(
            path("/foo"),
            Value::Number(42.into()),
        )]);
        check!(patch_ops == expected_patch);
    }

    #[test]
    fn test_diff_recursive_with_both_objects_should_remove() {
        let left = serde_json::json!({"foo": 43, "bar": 1});
        let right = serde_json::json!({"foo": 43});

        let patch_ops = diff_recursive(&left, &right, None, &Spath::default(), &Patch::default());

        let expected_patch = Patch::new(vec![PatchOp::remove(path("/bar"))]);
        check!(patch_ops == expected_patch);
    }

    #[test]
    fn test_diff_recursive_with_both_objects_should_add() {
        let left = serde_json::json!({"foo": 43});
        let right = serde_json::json!({"foo": 43, "bar": 1});

        let patch_ops = diff_recursive(&left, &right, None, &Spath::default(), &Patch::default());

        let expected_patch = Patch::new(vec![PatchOp::add(path("/bar"), Value::Number(1.into()))]);
        check!(patch_ops == expected_patch);
    }

    // NOTE: We currently do not handle move operations in diffing.
    // #[test]
    // fn test_diff_recursive_with_both_objects_should_move() {
    //     let left = serde_json::json!({"foo": {"bar": 1}});
    //     let right = serde_json::json!({"baz": {"bar": 1}});
    //
    //     let patch_ops = diff_recursive(&left, &right, None, &Spath::default(), &Patch::default());
    //
    //     let expected_patch = Patch::new(vec![PatchOp::move_op(
    //         path("/foo"),
    //         path("/baz"),
    //     )]);
    //     check!(patch_ops == expected_patch);
    // }

    #[test]
    fn test_diff_recursive_with_schema_remove_array_element() {
        let schema: serde_json::Value = serde_json::from_str(SIMPLE_SCHEMA).unwrap();
        let schema = Some(&schema);

        let left = serde_json::json!({"foo": [
            {"id": "abc", "count": 2},
            {"id": "bla", "count": 3},
        ]});
        let right = serde_json::json!({"foo": [
            {"id": "abc", "count": 2},
        ]});

        let patch_ops = diff_recursive(&left, &right, schema, &Spath::default(), &Patch::default());

        let expected_patch = Patch::new(vec![PatchOp::remove(path("/foo/[id=bla]"))]);
        check!(patch_ops[0] == expected_patch[0]);
    }

    #[test]
    fn test_diff_recursive_with_schema_add_array_element() {
        let schema: serde_json::Value = serde_json::from_str(SIMPLE_SCHEMA).unwrap();
        let schema = Some(&schema);

        let left = serde_json::json!({"foo": [
            {"id": "abc", "count": 2},
        ]});
        let right = serde_json::json!({"foo": [
            {"id": "abc", "count": 2},
            {"id": "bla", "count": 3},
        ]});

        let patch_ops = diff_recursive(&left, &right, schema, &Spath::default(), &Patch::default());

        let expected_patch = Patch::new(vec![PatchOp::add(
            path("/foo/[id=bla]"),
            serde_json::json!({"id": "bla", "count": 3}),
        )]);
        check!(patch_ops.len() == 1);
        check!(patch_ops[0] == expected_patch[0]);
    }

    #[test]
    fn test_diff_recursive_with_schema_modify_array_element() {
        let schema: serde_json::Value = serde_json::from_str(SIMPLE_SCHEMA).unwrap();
        let schema = Some(&schema);

        let left = serde_json::json!({"foo": [
            {"id": "abc", "count": 2},
            {"id": "bla", "count": 10},
        ]});
        let right = serde_json::json!({"foo": [
            {"id": "abc", "count": 2},
            {"id": "bla", "count": 3},
        ]});

        let patch_ops = diff_recursive(&left, &right, schema, &Spath::default(), &Patch::default());

        let expected_patch = Patch::new(vec![PatchOp::replace(
            path("/foo/[id=bla]/count"),
            Value::Number(3.into()),
        )]);
        check!(patch_ops[0] == expected_patch[0]);
    }

    #[test]
    fn test_diff_recursive_with_both_objects_add_and_remove() {
        let schema: serde_json::Value = serde_json::from_str(SIMPLE_SCHEMA).unwrap();
        let schema = Some(&schema);

        let left = serde_json::json!({"bar": 1, "foo": [
            {"id": "abc", "count": 2},
            {"id": "bla", "count": 3},
        ]});
        let right = serde_json::json!({"foo": [
            {"id": "abc", "count": 2},
            {"id": "bla", "count": 10},
        ]});

        let patch_ops = diff_recursive(&left, &right, schema, &Spath::default(), &Patch::default());

        let expected_patch = Patch::new(vec![
            PatchOp::replace(path("/foo/[id=bla]/count"), Value::Number(10.into())),
            PatchOp::remove(path("/bar")),
        ]);

        check!(patch_ops.len() == 2);
        check!(patch_ops[0] == expected_patch[0]);
        check!(patch_ops[1] == expected_patch[1]);
    }

    #[test]
    fn test_diff_recursive_without_schema_remove_array_element() {
        let left = serde_json::json!({"foo": [
            {"id": "abc", "count": 2},
            {"id": "bla", "count": 3},
        ]});
        let right = serde_json::json!({"foo": [
            {"id": "abc", "count": 2},
        ]});

        let patch_ops = diff_recursive(&left, &right, None, &Spath::default(), &Patch::default());

        let expected_patch = Patch::new(vec![PatchOp::remove(path("/foo/1"))]);
        check!(patch_ops.len() == 1);
        check!(patch_ops[0] == expected_patch[0]);
    }

    #[test]
    fn test_diff_recursive_without_schema_remove_first_array_element() {
        let left = serde_json::json!({"foo": [
            {"id": "abc", "count": 2},
            {"id": "bla", "count": 3},
        ]});
        let right = serde_json::json!({"foo": [
            {"id": "bla", "count": 3},
        ]});

        let patch_ops = diff_recursive(&left, &right, None, &Spath::default(), &Patch::default());

        // TODO: The result should be a just removal of the object at index 0
        // Currently the emitted patch is not optimal, but valid
        // We might want to optimize this in the future
        let expected_patch = Patch::new(vec![PatchOp::remove(path("/foo/0"))]);
        check!(patch_ops.len() == 1);
        check!(patch_ops[0] == expected_patch[0]);
    }

    #[test]
    fn test_diff_recursive_without_schema_replace_array_element() {
        let left = serde_json::json!({"foo": [
            {"id": "abc", "count": 2},
            {"id": "bla", "count": 3},
        ]});
        let right = serde_json::json!({"foo": [
            {"id": "abc", "count": 2},
            {"id": "lol", "count": 10},
        ]});

        let patch_ops = diff_recursive(&left, &right, None, &Spath::default(), &Patch::default());

        let expected_patch = Patch::new(vec![PatchOp::replace(
            path("/foo/1"),
            serde_json::json!({"id": "lol", "count": 10}),
        )]);
        check!(patch_ops.len() == 1);
        check!(patch_ops[0] == expected_patch[0]);
    }

    #[test]
    fn test_diff_recursive_without_schema_add_array_element() {
        let left = serde_json::json!({"foo": [
            {"id": "abc", "count": 2},
        ]});
        let right = serde_json::json!({"foo": [
            {"id": "abc", "count": 2},
            {"id": "lol", "count": 10},
        ]});

        let patch_ops = diff_recursive(&left, &right, None, &Spath::default(), &Patch::default());

        let expected_patch = Patch::new(vec![PatchOp::add(
            path("/foo/-"),
            serde_json::json!({"id": "lol", "count": 10}),
        )]);
        check!(patch_ops.len() == 1);
        check!(patch_ops[0] == expected_patch[0]);
    }

    #[test]
    fn test_diff_recursive_against_the_jsonpatch_spec_tests() {
        let test_cases = json_patch_tests::load_json_patch_test_cases_for_diff();

        // Collect all failures instead of failing immediately.
        let mut failures = Vec::new();

        for test_case in test_cases {
            match test_case {
                json_patch_tests::JsonPatchTestCase::Valid {
                    doc,
                    patch,
                    expected,
                    comment,
                } => {
                    let comment = comment.unwrap_or_default();

                    let patch_ops =
                        diff_recursive(&doc, &expected, None, &Spath::default(), &Patch::default());

                    let mut generated_patch_json = Vec::new();
                    for op in patch_ops.iter() {
                        let op_json = serde_json::to_value(op).unwrap();
                        generated_patch_json.push(op_json);
                    }

                    if generated_patch_json != patch {
                        // Record a human-friendly failure description.
                        failures.push(format!(
                            "Failed test case: {comment}\n  Actual patch:   {actual}\n  Expected patch: {expected}",
                            comment = comment,
                            actual = serde_json::to_string(&generated_patch_json).unwrap(),
                            expected = serde_json::to_string(&patch).unwrap(),
                        ));
                    }
                }
                json_patch_tests::JsonPatchTestCase::Failure { .. } => {
                    // No need to test failure cases for diffing as the defined failures are for
                    // patch application.
                }
            }
        }

        // Only fail once, after we've run all cases.
        if !failures.is_empty() {
            panic!(
                "jsonpatch spec compliance failed for {} case(s):\n\n{}\n\nNumber of failures: {}",
                failures.len(),
                failures.join("\n\n"),
                failures.len(),
            );
        }
    }

    #[test]
    fn diff_object_should_replace_the_entire_value_when_majority_of_fields_changed() {
        let left = serde_json::json!({
            "a": 1,
            "b": 2,
            "c": 3,
            "d": 4,
        });
        let right = serde_json::json!({
            "a": 10,
            "b": 20,
            "c": 30,
            "d": 4,
        });

        let patch_ops = diff_recursive(&left, &right, None, &Spath::default(), &Patch::default());

        let expected_patch = Patch::new(vec![PatchOp::replace(Spath::default(), right.clone())]);

        check!(patch_ops.len() == 1);
        check!(patch_ops == expected_patch);
    }

    #[test]
    fn diff_array_indexed_should_handle_pure_truncate() {
        let left = serde_json::json!(["a", "b", "c", "d"]);
        let right = serde_json::json!(["a", "b"]);

        let patch_ops = diff_recursive(&left, &right, None, &Spath::default(), &Patch::default());

        let expected_patch = Patch::new(vec![
            PatchOp::remove(path("/3")),
            PatchOp::remove(path("/2")),
        ]);

        check!(patch_ops.len() == 2);
        check!(patch_ops == expected_patch);
    }

    #[test]
    fn diff_array_indexed_should_handle_pure_append() {
        let left = serde_json::json!(["a", "b"]);
        let right = serde_json::json!(["a", "b", "c", "d"]);

        let patch_ops = diff_recursive(&left, &right, None, &Spath::default(), &Patch::default());

        let expected_patch = Patch::new(vec![
            PatchOp::add(path("/-"), serde_json::json!("c")),
            PatchOp::add(path("/-"), serde_json::json!("d")),
        ]);

        check!(patch_ops.len() == 2);
        check!(patch_ops == expected_patch);
    }

    #[test]
    fn diff_array_indexed_should_handle_pure_remove_from_front() {
        let left = serde_json::json!(["a", "b", "c", "d"]);
        let right = serde_json::json!(["c", "d"]);

        let patch_ops = diff_recursive(&left, &right, None, &Spath::default(), &Patch::default());

        let expected_patch = Patch::new(vec![
            PatchOp::remove(path("/0")),
            PatchOp::remove(path("/0")),
        ]);

        check!(patch_ops.len() == 2);
        check!(patch_ops == expected_patch);
    }

    #[test]
    fn diff_array_indexed_should_handle_pure_add_to_front() {
        let left = serde_json::json!(["c", "d"]);
        let right = serde_json::json!(["a", "b", "c", "d"]);

        let patch_ops = diff_recursive(&left, &right, None, &Spath::default(), &Patch::default());

        let expected_patch = Patch::new(vec![
            PatchOp::add(path("/0"), serde_json::json!("b")),
            PatchOp::add(path("/0"), serde_json::json!("a")),
        ]);

        check!(patch_ops.len() == 2);
        check!(patch_ops == expected_patch);
    }

    #[test]
    fn diff_array_indexed_should_handle_mixed_changes() {
        let left = serde_json::json!(["a", "b", "c"]);
        let right = serde_json::json!(["a", "x", "c", "d"]);

        let patch_ops = diff_recursive(&left, &right, None, &Spath::default(), &Patch::default());

        let expected_patch = Patch::new(vec![
            PatchOp::replace(path("/1"), serde_json::json!("x")),
            PatchOp::add(path("/-"), serde_json::json!("d")),
        ]);

        check!(patch_ops.len() == 2);
        check!(patch_ops == expected_patch);
    }
}
