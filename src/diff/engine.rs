use std::collections::{HashMap, HashSet, hash_map::Entry};

use serde_json::Value;

use crate::{
    diff::{
        DiffOptions,
        error::{DiffError, DiffErrorSummary},
        options::DiffGranularity,
    },
    path::Spath,
};

use super::Patch;

/// Name of the property in the JSON Schema that indicates the index key for arrays.
/// It's used to identify unique items in an array for diffing purposes.
pub(super) const HASH_KEY_PROP_NAME: &str = "x-spatch-indexKey";

pub(super) fn diff_recursive(
    left: &serde_json::Value,
    right: &serde_json::Value,
    options: DiffOptions,
    path_pos: &Spath,
    patch_ops: &Patch,
) -> (Patch, DiffErrorSummary) {
    match (left, right) {
        (Value::Object(left_map), Value::Object(right_map)) => {
            diff_object(left_map, right_map, options, path_pos, patch_ops)
        }
        (Value::Array(left_array), Value::Array(right_array)) => {
            diff_array(left_array, right_array, options, path_pos, patch_ops)
        }
        (left, right) if left == right => (Patch::default(), DiffErrorSummary::empty()), // Values are equal, no diff needed
        (_, right) => {
            let patch = super::PatchOp::replace(path_pos.clone(), right.clone());
            // patch_ops.push(patch.clone());

            (Patch::new(vec![patch]), DiffErrorSummary::empty())
        }
    }
}

fn diff_object(
    left_map: &serde_json::Map<String, Value>,
    right_map: &serde_json::Map<String, Value>,
    options: DiffOptions,
    path_pointer: &Spath,
    patch_ops: &Patch,
) -> (Patch, DiffErrorSummary) {
    let inner_patch = right_map
        .iter()
        .map(|(key, right_value)| {
            let sub_schema = options.property_schema(key);
            match left_map.get(key) {
                // If the key exists in both maps, recurse into the values
                Some(left_value) => {
                    let child_options = options.with_optional_schema(sub_schema);

                    let child_path = path_pointer.push(crate::path::Segment::Field(key.clone()));
                    diff_recursive(
                        left_value,
                        right_value,
                        child_options,
                        &child_path,
                        patch_ops,
                    )
                }
                // Otherwise, it's an addition
                None => {
                    let child_path = path_pointer.push(crate::path::Segment::Field(key.clone()));
                    let patch_op = super::PatchOp::add(child_path.clone(), right_value.clone());

                    (Patch::new(vec![patch_op]), DiffErrorSummary::empty())
                }
            }
        })
        .fold((Patch::default(), DiffErrorSummary::empty()), |acc, p| {
            (acc.0 + p.0, acc.1 + p.1)
        });

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

    let computed_patch = (inner_patch.0 + Patch::new(removals), inner_patch.1.clone());

    let inner_patch_size_bytes = serde_json::to_vec(&computed_patch.0).unwrap().len();
    let replace_patch_size_bytes = serde_json::to_vec(&replace_patch).unwrap().len();

    match options.granularity {
        DiffGranularity::Compact => {
            if !patch_contains_semantic_path(&computed_patch.0)
                && replace_patch_size_bytes < inner_patch_size_bytes
            {
                (replace_patch, inner_patch.1)
            } else {
                computed_patch
            }
        }
        DiffGranularity::Granular => computed_patch,
    }
}

fn patch_contains_semantic_path(patch: &Patch) -> bool {
    patch.iter().any(patch_op_contains_semantic_path)
}

fn patch_op_contains_semantic_path(op: &super::PatchOp) -> bool {
    match op {
        super::PatchOp::Add { path, .. }
        | super::PatchOp::Remove { path }
        | super::PatchOp::Replace { path, .. }
        | super::PatchOp::Test { path, .. } => path_contains_semantic_segment(path),
        super::PatchOp::Move { from, path } | super::PatchOp::Copy { from, path } => {
            path_contains_semantic_segment(from) || path_contains_semantic_segment(path)
        }
    }
}

fn path_contains_semantic_segment(path: &Spath) -> bool {
    path.into_iter()
        .any(|segment| matches!(segment, crate::path::Segment::Filter(_)))
}

fn diff_array(
    left: &[Value],
    right: &[Value],
    options: DiffOptions<'_>,
    path_pointer: &Spath,
    patch_ops: &Patch,
) -> (Patch, DiffErrorSummary) {
    // TODO: emit warning if the schema is missing an index key when the schema is provided
    let index_key = options.index_key().map(str::to_owned);

    match index_key {
        // If the schema specifies an index key, use keyed diffing
        Some(ref key) if options.schema.is_some() => {
            diff_array_keyed(left, right, key, options, path_pointer, patch_ops)
        }
        // Otherwise, use index based diffing
        _ => diff_array_indexed(left, right, options, path_pointer, patch_ops),
    }
}

fn diff_array_keyed(
    left: &[Value],
    right: &[Value],
    index_key: &str,
    options: DiffOptions,
    path_pointer: &Spath,
    patch_ops: &Patch,
) -> (Patch, DiffErrorSummary) {
    // Build maps: key -> element
    let (map_left, errors_left) = build_key_map(left, index_key, path_pointer);
    let (map_right, errors_right) = build_key_map(right, index_key, path_pointer);

    let keys_a: HashSet<_> = map_left.keys().cloned().collect();
    let keys_b: HashSet<_> = map_right.keys().cloned().collect();

    let mut removed_keys: Vec<_> = keys_a.difference(&keys_b).collect();
    removed_keys.sort_unstable();

    let mut added_keys: Vec<_> = keys_b.difference(&keys_a).collect();
    added_keys.sort_unstable();

    let mut modified_keys: Vec<_> = keys_a.intersection(&keys_b).collect();
    modified_keys.sort_unstable();

    // Removed elements
    let removed = removed_keys
        .into_iter()
        .map(|key| {
            let child_path = path_pointer.push_filter(index_key, key);
            Patch::new_with_op(super::PatchOp::remove(child_path.clone()))
        })
        .fold(Patch::default(), |acc, p| acc + p);

    // Added elements
    let added = added_keys
        .into_iter()
        .map(|key| {
            let child_path = path_pointer.push_filter(index_key, key);
            let val = &map_right[key];

            Patch::new_with_op(super::PatchOp::add(child_path.clone(), val.clone()))
        })
        .fold(Patch::default(), |acc, p| acc + p);

    let sub_schema = options.items_schema();
    let child_options = options.with_optional_schema(sub_schema);

    // Modified elements (same key in both)
    let modified = modified_keys
        .into_iter()
        .map(|key| {
            let child_path = path_pointer.push_filter(index_key, key);
            let value_left = &map_left[key];
            let value_right = &map_right[key];

            diff_recursive(
                value_left,
                value_right,
                child_options,
                &child_path,
                patch_ops,
            )
        })
        .fold((Patch::default(), DiffErrorSummary::empty()), |acc, p| {
            (acc.0 + p.0, acc.1 + p.1)
        });

    let patch = removed + added + modified.0;
    (
        patch,
        DiffErrorSummary::new(errors_left, errors_right) + modified.1,
    )
}

fn build_key_map(
    arr: &[Value],
    index_key: &str,
    path_pointer: &Spath,
) -> (HashMap<String, Value>, Vec<DiffError>) {
    let mut map = HashMap::new();
    let mut errors = Vec::new();
    for (i, item) in arr.iter().enumerate() {
        let current_path = path_pointer.push(crate::path::Segment::Field(format!("{}", i)));
        match item {
            Value::Object(obj) => match obj.get(index_key) {
                Some(value) => match index_key_value_to_filter(value) {
                    Some(key) => match map.entry(key) {
                        Entry::Occupied(entry) => {
                            errors.push(DiffError::duplicate_index_key(
                                &current_path,
                                index_key,
                                entry.key(),
                            ));
                        }
                        Entry::Vacant(entry) => {
                            entry.insert(item.clone());
                        }
                    },
                    None => {
                        errors.push(DiffError::non_string_index_key(&current_path, value));
                    }
                },
                None => {
                    errors.push(DiffError::missing_index_key(&current_path, index_key));
                }
            },
            _ => errors.push(DiffError::non_object_array_item(path_pointer, item)),
        }
    }
    (map, errors)
}

fn index_key_value_to_filter(value: &Value) -> Option<String> {
    match value {
        Value::String(s) => Some(s.clone()),
        Value::Number(n) => Some(n.to_string()),
        Value::Bool(b) => Some(b.to_string()),
        Value::Null | Value::Array(_) | Value::Object(_) => None,
    }
}

fn diff_array_indexed(
    left_array: &[Value],
    right_array: &[Value],
    options: DiffOptions,
    path_pointer: &Spath,
    patch_ops: &Patch,
) -> (Patch, DiffErrorSummary) {
    let len_left = left_array.len();
    let len_right = right_array.len();

    let item_options = if let Some(sub_schema) = options.items_schema() {
        options.with_optional_schema(Some(sub_schema))
    } else {
        options.without_schema()
    };

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
        return (patch, DiffErrorSummary::empty());
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
        return (patch, DiffErrorSummary::empty());
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
        return (patch, DiffErrorSummary::empty());
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
        return (patch, DiffErrorSummary::empty());
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
                item_options,
                &child_path,
                patch_ops,
            )
        })
        .fold((Patch::default(), DiffErrorSummary::empty()), |acc, p| {
            (acc.0 + p.0, acc.1 + p.1)
        });

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

    let patch = recursed.0 + removals + additions;
    let diff_errors = recursed.1;
    (patch, diff_errors)
}

#[cfg(test)]
mod tests {
    use assert2::{assert, check};

    use crate::diff::test_util::SIMPLE_SCHEMA;
    use crate::diff::test_util::json_patch_tests;
    use crate::diff::{DiffGranularity, PatchOp};

    use super::*;

    fn path(raw: &str) -> Spath {
        raw.try_into().unwrap()
    }

    #[test]
    fn test_diff_recursive_equal_values() {
        let left = serde_json::json!("foo");
        let right = serde_json::json!("foo");

        let (patch_ops, diff_errors) = diff_recursive(
            &left,
            &right,
            DiffOptions::new(),
            &Spath::default(),
            &Patch::default(),
        );

        // No patch operations should be generated for equal values
        check!(diff_errors.is_empty() == true);
        check!(patch_ops == Patch::default());
    }

    #[test]
    fn test_diff_recursive_non_equal_values() {
        let left = serde_json::json!("foo");
        let right = serde_json::json!("bar");

        let (patch_ops, diff_errors) = diff_recursive(
            &left,
            &right,
            DiffOptions::new(),
            &Spath::default(),
            &Patch::default(),
        );

        let expected_patch = Patch::new(vec![PatchOp::replace(Spath::default(), right.clone())]);

        check!(diff_errors.is_empty() == true);
        check!(patch_ops == expected_patch);
    }

    #[test]
    fn test_diff_recursive_with_left_string_and_right_object() {
        let left = serde_json::json!("foo");
        let right = serde_json::json!({"baz": 42});

        let (patch_ops, diff_errors) = diff_recursive(
            &left,
            &right,
            DiffOptions::new(),
            &Spath::default(),
            &Patch::default(),
        );

        let expected_patch = Patch::new(vec![PatchOp::replace(Spath::default(), right.clone())]);

        check!(diff_errors.is_empty() == true);
        check!(patch_ops == expected_patch);
    }

    #[test]
    fn test_diff_recursive_with_both_objects() {
        let left = serde_json::json!({"foo": 43});
        let right = serde_json::json!({"foo": 42});

        let (patch_ops, diff_errors) = diff_recursive(
            &left,
            &right,
            DiffOptions::new(),
            &Spath::default(),
            &Patch::default(),
        );

        let expected_patch = Patch::new(vec![PatchOp::replace(
            path("/foo"),
            Value::Number(42.into()),
        )]);

        check!(diff_errors.is_empty() == true);
        check!(patch_ops == expected_patch);
    }

    #[test]
    fn test_diff_recursive_with_both_objects_should_remove() {
        let left = serde_json::json!({"foo": 43, "bar": 1});
        let right = serde_json::json!({"foo": 43});

        let (patch_ops, diff_errors) = diff_recursive(
            &left,
            &right,
            DiffOptions::new(),
            &Spath::default(),
            &Patch::default(),
        );

        let expected_patch = Patch::new(vec![PatchOp::remove(path("/bar"))]);

        check!(diff_errors.is_empty() == true);
        check!(patch_ops == expected_patch);
    }

    #[test]
    fn test_diff_recursive_with_both_objects_should_add() {
        let left = serde_json::json!({"foo": 43});
        let right = serde_json::json!({"foo": 43, "bar": 1});

        let (patch_ops, diff_errors) = diff_recursive(
            &left,
            &right,
            DiffOptions::new(),
            &Spath::default(),
            &Patch::default(),
        );

        let expected_patch = Patch::new(vec![PatchOp::add(path("/bar"), Value::Number(1.into()))]);

        check!(diff_errors.is_empty() == true);
        check!(patch_ops == expected_patch);
    }

    #[test]
    fn compact_granularity_should_replace_parent_object_when_smaller() {
        let left = serde_json::json!({
            "long_field_name_a": 1,
            "long_field_name_b": 2,
            "long_field_name_c": 3,
        });
        let right = serde_json::json!({
            "long_field_name_a": 10,
            "long_field_name_b": 20,
            "long_field_name_c": 30,
        });

        let (patch_ops, diff_errors) = diff_recursive(
            &left,
            &right,
            DiffOptions::new().with_granularity(DiffGranularity::Compact),
            &Spath::default(),
            &Patch::default(),
        );

        let expected_patch = Patch::new(vec![PatchOp::replace(Spath::default(), right.clone())]);

        check!(diff_errors.is_empty() == true);
        check!(patch_ops == expected_patch);
    }

    #[test]
    fn granular_granularity_should_keep_nested_object_operations() {
        let left = serde_json::json!({
            "long_field_name_a": 1,
            "long_field_name_b": 2,
            "long_field_name_c": 3,
        });
        let right = serde_json::json!({
            "long_field_name_a": 10,
            "long_field_name_b": 20,
            "long_field_name_c": 30,
        });

        let (patch_ops, diff_errors) = diff_recursive(
            &left,
            &right,
            DiffOptions::new().granular(),
            &Spath::default(),
            &Patch::default(),
        );

        let expected_patch = Patch::new(vec![
            PatchOp::replace(path("/long_field_name_a"), Value::Number(10.into())),
            PatchOp::replace(path("/long_field_name_b"), Value::Number(20.into())),
            PatchOp::replace(path("/long_field_name_c"), Value::Number(30.into())),
        ]);

        check!(diff_errors.is_empty() == true);
        check!(patch_ops == expected_patch);
    }

    #[test]
    fn patch_contains_semantic_path_should_check_move_and_copy_from_paths() {
        let destination_path = path("/tracks/0/levels/0");

        let move_patch = Patch::new(vec![PatchOp::move_op(
            path("/tracks/[id=free]/levels/[id=1]"),
            destination_path.clone(),
        )]);
        check!(patch_contains_semantic_path(&move_patch));

        let copy_patch = Patch::new(vec![PatchOp::copy(
            path("/tracks/[id=free]/levels/[id=1]"),
            destination_path,
        )]);
        check!(patch_contains_semantic_path(&copy_patch));
    }

    #[test]
    fn compact_granularity_should_preserve_schema_aware_semantic_paths() {
        let schema = serde_json::json!({
            "properties": {
                "tracks": {
                    "type": "array",
                    "x-spatch-indexKey": "id",
                    "items": {
                        "type": "object",
                        "properties": {
                            "id": { "type": "string" },
                            "levels": {
                                "type": "array",
                                "x-spatch-indexKey": "id",
                                "items": {
                                    "type": "object",
                                    "properties": {
                                        "id": { "type": "integer" },
                                        "xp": { "type": "integer" }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        });
        let left = serde_json::json!({
            "tracks": [{
                "id": "free",
                "levels": [
                    { "id": 1, "xp": 100 },
                    { "id": 2, "xp": 200 },
                    { "id": 3, "xp": 300 }
                ]
            }]
        });
        let right = serde_json::json!({
            "tracks": [{
                "id": "free",
                "levels": [
                    { "id": 1, "xp": 1000 },
                    { "id": 2, "xp": 2000 },
                    { "id": 3, "xp": 3000 }
                ]
            }]
        });

        let (patch_ops, diff_errors) = diff_recursive(
            &left,
            &right,
            DiffOptions::new().with_schema(&schema).compact(),
            &Spath::default(),
            &Patch::default(),
        );

        let expected_patch = Patch::new(vec![
            PatchOp::replace(
                path("/tracks/[id=free]/levels/[id=1]/xp"),
                serde_json::json!(1000),
            ),
            PatchOp::replace(
                path("/tracks/[id=free]/levels/[id=2]/xp"),
                serde_json::json!(2000),
            ),
            PatchOp::replace(
                path("/tracks/[id=free]/levels/[id=3]/xp"),
                serde_json::json!(3000),
            ),
        ]);

        check!(diff_errors.is_empty() == true);
        check!(patch_ops == expected_patch);
    }

    #[test]
    fn schema_should_not_leak_to_object_property_without_schema() {
        let schema = serde_json::json!({
            "x-spatch-indexKey": "id",
            "properties": {}
        });
        let left = serde_json::json!({
            "items": [{"name": "old"}]
        });
        let right = serde_json::json!({
            "items": [{"name": "new"}]
        });

        let (patch_ops, diff_errors) = diff_recursive(
            &left,
            &right,
            DiffOptions::new().with_schema(&schema).granular(),
            &Spath::default(),
            &Patch::default(),
        );

        let expected_patch = Patch::new(vec![PatchOp::replace(
            path("/items/0/name"),
            serde_json::json!("new"),
        )]);

        check!(diff_errors.is_empty() == true);
        check!(patch_ops == expected_patch);
    }

    #[test]
    fn keyed_array_schema_should_not_leak_to_items_without_items_schema() {
        let schema = serde_json::json!({
            "x-spatch-indexKey": "id"
        });
        let left = serde_json::json!([{
            "id": "a",
            "nested": [{"name": "old"}]
        }]);
        let right = serde_json::json!([{
            "id": "a",
            "nested": [{"name": "new"}]
        }]);

        let (patch_ops, diff_errors) = diff_recursive(
            &left,
            &right,
            DiffOptions::new().with_schema(&schema).granular(),
            &Spath::default(),
            &Patch::default(),
        );

        let expected_patch = Patch::new(vec![PatchOp::replace(
            path("/[id=a]/nested/0/name"),
            serde_json::json!("new"),
        )]);

        check!(diff_errors.is_empty() == true);
        check!(patch_ops == expected_patch);
    }

    #[test]
    fn schema_ref_items_should_use_nested_array_index_keys() {
        let schema = serde_json::json!({
            "properties": {
                "tracks": {
                    "type": "array",
                    "x-spatch-indexKey": "id",
                    "items": { "$ref": "#/$defs/track" }
                }
            },
            "$defs": {
                "track": {
                    "type": "object",
                    "properties": {
                        "id": { "type": "string" },
                        "levels": {
                            "type": "array",
                            "x-spatch-indexKey": "id",
                            "items": { "$ref": "#/$defs/level" }
                        }
                    }
                },
                "level": {
                    "type": "object",
                    "properties": {
                        "id": { "type": "string" },
                        "xp": { "type": "integer" }
                    }
                }
            }
        });
        let left = serde_json::json!({
            "tracks": [{
                "id": "free",
                "levels": [{ "id": "1", "xp": 100 }]
            }]
        });
        let right = serde_json::json!({
            "tracks": [{
                "id": "free",
                "levels": [{ "id": "1", "xp": 200 }]
            }]
        });

        let (patch_ops, diff_errors) = diff_recursive(
            &left,
            &right,
            DiffOptions::new().with_schema(&schema).granular(),
            &Spath::default(),
            &Patch::default(),
        );

        let expected_patch = Patch::new(vec![PatchOp::replace(
            path("/tracks/[id=free]/levels/[id=1]/xp"),
            serde_json::json!(200),
        )]);

        check!(diff_errors.is_empty() == true);
        check!(patch_ops == expected_patch);
    }

    #[test]
    fn schema_ref_items_should_use_deeply_nested_array_index_keys() {
        let schema = serde_json::json!({
            "properties": {
                "tracks": {
                    "type": "array",
                    "x-spatch-indexKey": "id",
                    "items": { "$ref": "#/$defs/track" }
                }
            },
            "$defs": {
                "track": {
                    "type": "object",
                    "properties": {
                        "id": { "type": "string" },
                        "levels": {
                            "type": "array",
                            "x-spatch-indexKey": "id",
                            "items": { "$ref": "#/$defs/level" }
                        }
                    }
                },
                "level": {
                    "type": "object",
                    "properties": {
                        "id": { "type": "string" },
                        "rewards": {
                            "type": "array",
                            "x-spatch-indexKey": "id",
                            "items": { "$ref": "#/$defs/reward" }
                        }
                    }
                },
                "reward": {
                    "type": "object",
                    "properties": {
                        "id": { "type": "string" },
                        "amount": { "type": "integer" }
                    }
                }
            }
        });
        let left = serde_json::json!({
            "tracks": [{
                "id": "free",
                "levels": [{
                    "id": "1",
                    "rewards": [{ "id": "reward-1", "amount": 100 }]
                }]
            }]
        });
        let right = serde_json::json!({
            "tracks": [{
                "id": "free",
                "levels": [{
                    "id": "1",
                    "rewards": [{ "id": "reward-1", "amount": 200 }]
                }]
            }]
        });

        let (patch_ops, diff_errors) = diff_recursive(
            &left,
            &right,
            DiffOptions::new().with_schema(&schema).granular(),
            &Spath::default(),
            &Patch::default(),
        );

        let expected_patch = Patch::new(vec![PatchOp::replace(
            path("/tracks/[id=free]/levels/[id=1]/rewards/[id=reward-1]/amount"),
            serde_json::json!(200),
        )]);

        check!(diff_errors.is_empty() == true);
        check!(patch_ops == expected_patch);
    }

    #[test]
    fn schema_index_key_should_accept_numeric_values() {
        let schema = serde_json::json!({
            "properties": {
                "levels": {
                    "type": "array",
                    "x-spatch-indexKey": "id",
                    "items": {
                        "type": "object",
                        "properties": {
                            "id": { "type": "integer" },
                            "xp": { "type": "integer" }
                        }
                    }
                }
            }
        });
        let left = serde_json::json!({
            "levels": [{ "id": 1, "xp": 100 }]
        });
        let right = serde_json::json!({
            "levels": [{ "id": 1, "xp": 200 }]
        });

        let (patch_ops, diff_errors) = diff_recursive(
            &left,
            &right,
            DiffOptions::new().with_schema(&schema).granular(),
            &Spath::default(),
            &Patch::default(),
        );

        let expected_patch = Patch::new(vec![PatchOp::replace(
            path("/levels/[id=1]/xp"),
            serde_json::json!(200),
        )]);

        check!(diff_errors.is_empty() == true);
        check!(patch_ops == expected_patch);
    }

    // NOTE: We currently do not handle move operations in diffing.
    // #[test]
    // fn test_diff_recursive_with_both_objects_should_move() {
    //     let left = serde_json::json!({"foo": {"bar": 1}});
    //     let right = serde_json::json!({"baz": {"bar": 1}});
    //
    //     let (patch_ops, diff_errors) = diff_recursive(&left, &right, None, &Spath::default(), &Patch::default());
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
        let options = DiffOptions::new().with_schema(&schema);

        let left = serde_json::json!({"foo": [
            {"id": "abc", "count": 2},
            {"id": "bla", "count": 3},
        ]});
        let right = serde_json::json!({"foo": [
            {"id": "abc", "count": 2},
        ]});

        let (patch_ops, diff_errors) =
            diff_recursive(&left, &right, options, &Spath::default(), &Patch::default());

        let expected_patch = Patch::new(vec![PatchOp::remove(path("/foo/[id=bla]"))]);

        check!(diff_errors.is_empty() == true);
        check!(patch_ops[0] == expected_patch[0]);
    }

    #[test]
    fn test_diff_recursive_with_schema_add_array_element() {
        let schema: serde_json::Value = serde_json::from_str(SIMPLE_SCHEMA).unwrap();
        let options = DiffOptions::new().with_schema(&schema);

        let left = serde_json::json!({"foo": [
            {"id": "abc", "count": 2},
        ]});
        let right = serde_json::json!({"foo": [
            {"id": "abc", "count": 2},
            {"id": "bla", "count": 3},
        ]});

        let (patch_ops, diff_errors) =
            diff_recursive(&left, &right, options, &Spath::default(), &Patch::default());

        let expected_patch = Patch::new(vec![PatchOp::add(
            path("/foo/[id=bla]"),
            serde_json::json!({"id": "bla", "count": 3}),
        )]);
        check!(diff_errors.is_empty() == true);
        check!(patch_ops.len() == 1);
        check!(patch_ops[0] == expected_patch[0]);
    }

    #[test]
    fn test_diff_recursive_with_schema_modify_array_element() {
        let schema: serde_json::Value = serde_json::from_str(SIMPLE_SCHEMA).unwrap();
        let options = DiffOptions::new().with_schema(&schema);

        let left = serde_json::json!({"foo": [
            {"id": "abc", "count": 2},
            {"id": "bla", "count": 10},
        ]});
        let right = serde_json::json!({"foo": [
            {"id": "abc", "count": 2},
            {"id": "bla", "count": 3},
        ]});

        let (patch_ops, diff_errors) =
            diff_recursive(&left, &right, options, &Spath::default(), &Patch::default());

        let expected_patch = Patch::new(vec![PatchOp::replace(
            path("/foo/[id=bla]/count"),
            Value::Number(3.into()),
        )]);

        check!(diff_errors.is_empty() == true);
        check!(patch_ops[0] == expected_patch[0]);
    }

    #[test]
    fn diff_with_schema_and_not_matching_index_key_should_fail() {
        let schema: serde_json::Value = serde_json::from_str(SIMPLE_SCHEMA).unwrap();
        let options = DiffOptions::new().with_schema(&schema);

        let left = serde_json::json!({"foo": [
            {"key": "abc", "count": 2},
            {"key": "bla", "count": 10},
        ]});
        let right = serde_json::json!({"foo": [
            {"key": "abc", "count": 2},
            {"key": "bla", "count": 3},
        ]});

        let (_patch_ops, diff_errors) =
            diff_recursive(&left, &right, options, &Spath::default(), &Patch::default());

        check!(diff_errors.left.len() == 2);
        check!(diff_errors.right.len() == 2);
        check!(diff_errors.left[0] == DiffError::missing_index_key(&path("/foo/0"), "id"));
        check!(diff_errors.left[1] == DiffError::missing_index_key(&path("/foo/1"), "id"));
        check!(diff_errors.right[0] == DiffError::missing_index_key(&path("/foo/0"), "id"));
        check!(diff_errors.right[1] == DiffError::missing_index_key(&path("/foo/1"), "id"));
    }

    #[test]
    fn diff_with_schema_and_duplicate_index_key_should_fail() {
        let schema: serde_json::Value = serde_json::from_str(SIMPLE_SCHEMA).unwrap();
        let options = DiffOptions::new().with_schema(&schema);

        let left = serde_json::json!({"foo": [
            {"id": "abc", "count": 2},
            {"id": "abc", "count": 10},
        ]});
        let right = serde_json::json!({"foo": [
            {"id": "abc", "count": 2},
            {"id": "abc", "count": 3},
        ]});

        let (_patch_ops, diff_errors) =
            diff_recursive(&left, &right, options, &Spath::default(), &Patch::default());

        check!(diff_errors.left.len() == 1);
        check!(diff_errors.right.len() == 1);
        check!(diff_errors.left[0] == DiffError::duplicate_index_key(&path("/foo/1"), "id", "abc"));
        check!(
            diff_errors.right[0] == DiffError::duplicate_index_key(&path("/foo/1"), "id", "abc")
        );
    }

    #[test]
    fn diff_with_schema_and_unrepresentable_index_key_values_should_fail() {
        let schema: serde_json::Value = serde_json::from_str(SIMPLE_SCHEMA).unwrap();
        let options = DiffOptions::new().with_schema(&schema);

        let left = serde_json::json!({"foo": [
            {"id": {"nested": "object"}, "count": 2},
            {"id": ["array"], "count": 3},
            {"id": null, "count": 4},
        ]});
        let right = left.clone();

        let (_patch_ops, diff_errors) =
            diff_recursive(&left, &right, options, &Spath::default(), &Patch::default());

        let expected_errors = vec![
            DiffError::non_string_index_key(
                &path("/foo/0"),
                &serde_json::json!({"nested": "object"}),
            ),
            DiffError::non_string_index_key(&path("/foo/1"), &serde_json::json!(["array"])),
            DiffError::non_string_index_key(&path("/foo/2"), &serde_json::json!(null)),
        ];

        check!(diff_errors.left == expected_errors);
        check!(diff_errors.right == expected_errors);
    }

    #[test]
    fn diff_with_schema_and_non_object_array_items_should_fail() {
        let schema: serde_json::Value = serde_json::from_str(SIMPLE_SCHEMA).unwrap();
        let options = DiffOptions::new().with_schema(&schema);

        let left = serde_json::json!({"foo": ["left-only"]});
        let right = serde_json::json!({"foo": ["right-only"]});

        assert!(let Err(e) = crate::diff::diff(&left, &right, options));
        check!(
            e.to_string()
                == "Expected array items at path /foo to be objects for schema-aware diffing, but found string"
        );
    }

    #[test]
    fn test_diff_recursive_with_both_objects_add_and_remove() {
        let schema: serde_json::Value = serde_json::from_str(SIMPLE_SCHEMA).unwrap();
        let options = DiffOptions::new().with_schema(&schema);

        let left = serde_json::json!({"bar": 1, "foo": [
            {"id": "abc", "count": 2},
            {"id": "bla", "count": 3},
        ]});
        let right = serde_json::json!({"foo": [
            {"id": "abc", "count": 2},
            {"id": "bla", "count": 10},
        ]});

        let (patch_ops, diff_errors) =
            diff_recursive(&left, &right, options, &Spath::default(), &Patch::default());

        let expected_patch = Patch::new(vec![
            PatchOp::replace(path("/foo/[id=bla]/count"), Value::Number(10.into())),
            PatchOp::remove(path("/bar")),
        ]);

        check!(diff_errors.is_empty() == true);
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

        let (patch_ops, diff_errors) = diff_recursive(
            &left,
            &right,
            DiffOptions::new(),
            &Spath::default(),
            &Patch::default(),
        );

        let expected_patch = Patch::new(vec![PatchOp::remove(path("/foo/1"))]);

        check!(diff_errors.is_empty() == true);
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

        let (patch_ops, diff_errors) = diff_recursive(
            &left,
            &right,
            DiffOptions::new(),
            &Spath::default(),
            &Patch::default(),
        );

        // TODO: The result should be a just removal of the object at index 0
        // Currently the emitted patch is not optimal, but valid
        // We might want to optimize this in the future
        let expected_patch = Patch::new(vec![PatchOp::remove(path("/foo/0"))]);

        check!(diff_errors.is_empty() == true);
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

        let (patch_ops, diff_errors) = diff_recursive(
            &left,
            &right,
            DiffOptions::new(),
            &Spath::default(),
            &Patch::default(),
        );

        let expected_patch = Patch::new(vec![PatchOp::replace(
            path("/foo/1"),
            serde_json::json!({"id": "lol", "count": 10}),
        )]);

        check!(diff_errors.is_empty() == true);
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

        let (patch_ops, diff_errors) = diff_recursive(
            &left,
            &right,
            DiffOptions::new(),
            &Spath::default(),
            &Patch::default(),
        );

        let expected_patch = Patch::new(vec![PatchOp::add(
            path("/foo/-"),
            serde_json::json!({"id": "lol", "count": 10}),
        )]);

        check!(diff_errors.is_empty() == true);
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

                    let (patch_ops, _diff_errors) = diff_recursive(
                        &doc,
                        &expected,
                        DiffOptions::new(),
                        &Spath::default(),
                        &Patch::default(),
                    );

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

        let (patch_ops, diff_errors) = diff_recursive(
            &left,
            &right,
            DiffOptions::new(),
            &Spath::default(),
            &Patch::default(),
        );

        let expected_patch = Patch::new(vec![PatchOp::replace(Spath::default(), right.clone())]);

        check!(diff_errors.is_empty() == true);
        check!(patch_ops.len() == 1);
        check!(patch_ops == expected_patch);
    }

    #[test]
    fn diff_object_should_return_granular_list_of_patches_when_majority_of_fields_changed_and_granular_is_used()
     {
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

        let (patch_ops, diff_errors) = diff_recursive(
            &left,
            &right,
            DiffOptions::new().granular(),
            &Spath::default(),
            &Patch::default(),
        );

        let expected_patch = Patch::new(vec![
            PatchOp::replace(path("/a"), serde_json::json!(10)),
            PatchOp::replace(path("/b"), serde_json::json!(20)),
            PatchOp::replace(path("/c"), serde_json::json!(30)),
        ]);

        check!(diff_errors.is_empty() == true);
        check!(patch_ops.len() == 3);
        check!(patch_ops == expected_patch);
    }

    #[test]
    fn diff_array_indexed_should_handle_pure_truncate() {
        let left = serde_json::json!(["a", "b", "c", "d"]);
        let right = serde_json::json!(["a", "b"]);

        let (patch_ops, diff_errors) = diff_recursive(
            &left,
            &right,
            DiffOptions::new(),
            &Spath::default(),
            &Patch::default(),
        );

        let expected_patch = Patch::new(vec![
            PatchOp::remove(path("/3")),
            PatchOp::remove(path("/2")),
        ]);

        check!(diff_errors.is_empty() == true);
        check!(patch_ops.len() == 2);
        check!(patch_ops == expected_patch);
    }

    #[test]
    fn diff_array_indexed_should_handle_pure_append() {
        let left = serde_json::json!(["a", "b"]);
        let right = serde_json::json!(["a", "b", "c", "d"]);

        let (patch_ops, diff_errors) = diff_recursive(
            &left,
            &right,
            DiffOptions::new(),
            &Spath::default(),
            &Patch::default(),
        );

        let expected_patch = Patch::new(vec![
            PatchOp::add(path("/-"), serde_json::json!("c")),
            PatchOp::add(path("/-"), serde_json::json!("d")),
        ]);

        check!(diff_errors.is_empty() == true);
        check!(patch_ops.len() == 2);
        check!(patch_ops == expected_patch);
    }

    #[test]
    fn diff_array_indexed_should_handle_pure_remove_from_front() {
        let left = serde_json::json!(["a", "b", "c", "d"]);
        let right = serde_json::json!(["c", "d"]);

        let (patch_ops, diff_errors) = diff_recursive(
            &left,
            &right,
            DiffOptions::new(),
            &Spath::default(),
            &Patch::default(),
        );

        let expected_patch = Patch::new(vec![
            PatchOp::remove(path("/0")),
            PatchOp::remove(path("/0")),
        ]);

        check!(diff_errors.is_empty() == true);
        check!(patch_ops.len() == 2);
        check!(patch_ops == expected_patch);
    }

    #[test]
    fn diff_array_indexed_should_handle_pure_add_to_front() {
        let left = serde_json::json!(["c", "d"]);
        let right = serde_json::json!(["a", "b", "c", "d"]);

        let (patch_ops, diff_errors) = diff_recursive(
            &left,
            &right,
            DiffOptions::new(),
            &Spath::default(),
            &Patch::default(),
        );

        let expected_patch = Patch::new(vec![
            PatchOp::add(path("/0"), serde_json::json!("b")),
            PatchOp::add(path("/0"), serde_json::json!("a")),
        ]);

        check!(diff_errors.is_empty() == true);
        check!(patch_ops.len() == 2);
        check!(patch_ops == expected_patch);
    }

    #[test]
    fn diff_array_indexed_should_handle_mixed_changes() {
        let left = serde_json::json!(["a", "b", "c"]);
        let right = serde_json::json!(["a", "x", "c", "d"]);

        let (patch_ops, diff_errors) = diff_recursive(
            &left,
            &right,
            DiffOptions::new(),
            &Spath::default(),
            &Patch::default(),
        );

        let expected_patch = Patch::new(vec![
            PatchOp::replace(path("/1"), serde_json::json!("x")),
            PatchOp::add(path("/-"), serde_json::json!("d")),
        ]);

        check!(diff_errors.is_empty() == true);
        check!(patch_ops.len() == 2);
        check!(patch_ops == expected_patch);
    }
}
