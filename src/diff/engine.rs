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
    path_pos: &mut Spath,
    patch_ops: &mut Patch,
) {
    match (left, right) {
        (Value::Object(left_map), Value::Object(right_map)) => {
            diff_object(left_map, right_map, schema, path_pos, patch_ops)
        }
        (Value::Array(left_array), Value::Array(right_array)) => {
            diff_array(left_array, right_array, schema, path_pos, patch_ops)
        }
        (left, right) if left == right => {} // Values are equal, no diff needed
        (_, right) => patch_ops.push(super::PatchOp::replace(path_pos.clone(), right.clone())),
    }
}

fn diff_object(
    left_map: &serde_json::Map<String, Value>,
    right_map: &serde_json::Map<String, Value>,
    schema: Option<&serde_json::Value>,
    path_pointer: &mut Spath,
    patch_ops: &mut Patch,
) {
    for (key, right_value) in right_map {
        let sub_schema = schema.and_then(|s| s.get("properties").and_then(|props| props.get(key)));
        match left_map.get(key) {
            // If the key exists in both maps, recurse into the values
            Some(left_value) => {
                path_pointer.push(crate::path::Segment::Field(key.clone()));
                diff_recursive(left_value, right_value, sub_schema, path_pointer, patch_ops);
                path_pointer.pop();
            }
            // Otherwise, it's an addition
            None => {
                path_pointer.push(crate::path::Segment::Field(key.clone()));
                patch_ops.push(super::PatchOp::add(
                    path_pointer.clone(),
                    right_value.clone(),
                ));
                path_pointer.pop();
            }
        }
    }

    for key in left_map.keys() {
        if !right_map.contains_key(key) {
            path_pointer.push(crate::path::Segment::Field(key.clone()));
            patch_ops.push(super::PatchOp::remove(path_pointer.clone()));
            path_pointer.pop();
        }
    }
}

fn diff_array(
    left: &Vec<Value>,
    right: &Vec<Value>,
    schema: Option<&Value>,
    path_pointer: &mut Spath,
    patch_ops: &mut Patch,
) {
    // TODO: emit warning if the schema is missing an index key when the schema is provided
    let index_key = schema.and_then(|s| {
        s.get(HASH_KEY_PROP_NAME)
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
    });
    match (schema, index_key) {
        // If the schema specifies an index key, use keyed diffing
        (Some(schema), Some(ref key)) => {
            diff_array_keyed(left, right, key, schema, path_pointer, patch_ops);
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
    path_pointer: &mut Spath,
    patch_ops: &mut Patch,
) {
    // Build maps: key -> element
    let map_left = build_key_map(left, index_key);
    let map_right = build_key_map(right, index_key);

    let keys_a: HashSet<_> = map_left.keys().cloned().collect();
    let keys_b: HashSet<_> = map_right.keys().cloned().collect();

    // Removed elements
    for key in keys_a.difference(&keys_b) {
        path_pointer.push_filter(index_key, key);
        patch_ops.push(super::PatchOp::remove(path_pointer.clone()));
        path_pointer.pop();
    }

    // Added elements
    for key in keys_b.difference(&keys_a) {
        path_pointer.push_filter(index_key, key);
        let val = &map_right[key];
        patch_ops.push(super::PatchOp::add(path_pointer.clone(), val.clone()));
        path_pointer.pop();
    }

    // Modified elements (same key in both)
    let item_schema = schema.get("items");
    for key in keys_a.intersection(&keys_b) {
        path_pointer.push_filter(index_key, key);
        let va = &map_left[key];
        let vb = &map_right[key];

        diff_recursive(va, vb, item_schema, path_pointer, patch_ops);

        path_pointer.pop();
    }

    // Implementation for keyed arrays (not provided here)
}

fn build_key_map<'a>(arr: &'a [Value], index_key: &str) -> HashMap<String, Value> {
    let mut map = HashMap::new();
    for item in arr {
        if let Value::Object(obj) = item {
            if let Some(Value::String(k)) = obj.get(index_key) {
                // Last one wins if there are duplicates; you might want to error instead.
                map.insert(k.clone(), item.clone());
            }
        }
    }
    map
}

fn diff_array_indexed(
    left_array: &Vec<Value>,
    right_array: &Vec<Value>,
    schema: Option<&Value>,
    path_pointer: &mut Spath,
    patch_ops: &mut Patch,
) {
    // Implementation for indexed arrays (not provided here)
}

struct Report {
    failures: Vec<String>,
    warnings: Vec<String>,
}

#[cfg(test)]
mod tests {
    use assert2::check;

    use crate::diff::PatchOp;
    use crate::diff::test_util::SIMPLE_SCHEMA;

    use super::*;

    fn path(raw: &str) -> Spath {
        raw.try_into().unwrap()
    }

    #[test]
    fn test_diff_recursive_equal_values() {
        let left = serde_json::json!("foo");
        let right = serde_json::json!("foo");

        let mut path_pos = Spath::default();
        let mut patch_ops = Patch::default();

        diff_recursive(&left, &right, None, &mut path_pos, &mut patch_ops);

        // No patch operations should be generated for equal values
        check!(patch_ops == Patch::default());
    }

    #[test]
    fn test_diff_recursive_non_equal_values() {
        let left = serde_json::json!("foo");
        let right = serde_json::json!("bar");

        let mut path_pos = Spath::default();
        let mut patch_ops = Patch::default();

        diff_recursive(&left, &right, None, &mut path_pos, &mut patch_ops);

        let expected_patch = Patch::new(vec![PatchOp::replace(path_pos.clone(), right.clone())]);
        check!(patch_ops == expected_patch);
    }

    #[test]
    fn test_diff_recursive_with_left_string_and_right_object() {
        let left = serde_json::json!("foo");
        let right = serde_json::json!({"baz": 42});

        let mut path_pos = Spath::default();
        let mut patch_ops = Patch::default();

        diff_recursive(&left, &right, None, &mut path_pos, &mut patch_ops);

        let expected_patch = Patch::new(vec![PatchOp::replace(path_pos.clone(), right.clone())]);
        check!(patch_ops == expected_patch);
    }

    #[test]
    fn test_diff_recursive_with_both_objects() {
        let left = serde_json::json!({"foo": 43});
        let right = serde_json::json!({"foo": 42});

        let mut path_pos = Spath::default();
        let mut patch_ops = Patch::default();

        diff_recursive(&left, &right, None, &mut path_pos, &mut patch_ops);

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

        let mut path_pos = Spath::default();
        let mut patch_ops = Patch::default();

        diff_recursive(&left, &right, None, &mut path_pos, &mut patch_ops);

        let expected_patch = Patch::new(vec![PatchOp::remove(path("/bar"))]);
        check!(patch_ops == expected_patch);
    }

    #[test]
    fn test_diff_recursive_with_both_objects_should_add() {
        let left = serde_json::json!({"foo": 43});
        let right = serde_json::json!({"foo": 43, "bar": 1});

        let mut path_pos = Spath::default();
        let mut patch_ops = Patch::default();

        diff_recursive(&left, &right, None, &mut path_pos, &mut patch_ops);

        let expected_patch = Patch::new(vec![PatchOp::add(path("/bar"), Value::Number(1.into()))]);
        check!(patch_ops == expected_patch);
    }

    // NOTE: We currently do not handle move operations in diffing.
    // #[test]
    // fn test_diff_recursive_with_both_objects_should_move() {
    //     let left = serde_json::json!({"foo": {"bar": 1}});
    //     let right = serde_json::json!({"baz": {"bar": 1}});
    //
    //     let mut path_pos = Spath::default();
    //     let mut patch_ops = Patch::default();
    //
    //     diff_recursive(&left, &right, None, &mut path_pos, &mut patch_ops);
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

        let mut path_pos = Spath::default();
        let mut patch_ops = Patch::default();

        diff_recursive(&left, &right, schema, &mut path_pos, &mut patch_ops);

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

        let mut path_pos = Spath::default();
        let mut patch_ops = Patch::default();

        diff_recursive(&left, &right, schema, &mut path_pos, &mut patch_ops);

        let expected_patch = Patch::new(vec![PatchOp::add(
            path("/foo/[id=bla]"),
            serde_json::json!({"id": "bla", "count": 3}),
        )]);
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

        let mut path_pos = Spath::default();
        let mut patch_ops = Patch::default();

        diff_recursive(&left, &right, schema, &mut path_pos, &mut patch_ops);

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

        let mut path_pos = Spath::default();
        let mut patch_ops = Patch::default();

        diff_recursive(&left, &right, schema, &mut path_pos, &mut patch_ops);

        let expected_patch = Patch::new(vec![
            PatchOp::replace(path("/foo/[id=bla]/count"), Value::Number(10.into())),
            PatchOp::remove(path("/bar")),
        ]);

        check!(patch_ops.len() == 2);
        check!(patch_ops[0] == expected_patch[0]);
        check!(patch_ops[1] == expected_patch[1]);
    }
}
