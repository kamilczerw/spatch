//! Generate JSON Patch diffs, with optional schema-aware array paths.
//!
//! The diff API produces JSON Patch operations while giving you two tools that
//! make patches more useful in real applications:
//!
//! - [`DiffOptions::with_schema`] enables semantic array paths such as
//!   `/users/[id=u-1]/name`, so patches talk about the item that changed rather
//!   than the position it happened to occupy. Schema-aware diffing follows local
//!   JSON Schema `$ref`s when walking `properties` and `items`, so nested arrays
//!   can produce paths like `/levels/[id=1]/xp` and
//!   `/tracks/[id=free]/levels/[id=1]/rewards/[id=reward-1]/amount`.
//! - [`DiffOptions::granular`] keeps object changes as nested operations when
//!   you want readable, review-friendly diffs. The default compact mode still
//!   keeps payloads small by collapsing large object changes when a parent
//!   replacement is shorter.
//!
//! # Example: a stable patch for an array item
//!
//! ```rust
//! use serde_json::json;
//! use spatch::diff::{diff, DiffOptions};
//!
//! let schema = json!({
//!     "properties": {
//!         "users": {
//!             "indexKey": "id",
//!             "items": {
//!                 "properties": {
//!                     "name": {}
//!                 }
//!             }
//!         }
//!     }
//! });
//!
//! let before = json!({
//!     "users": [
//!         {"id": "u-1", "name": "Ada"},
//!         {"id": "u-2", "name": "Grace"}
//!     ]
//! });
//! let after = json!({
//!     "users": [
//!         {"id": "u-2", "name": "Grace Hopper"},
//!         {"id": "u-1", "name": "Ada"}
//!     ]
//! });
//!
//! let patch = diff(&before, &after, DiffOptions::new().with_schema(&schema).granular())
//!     .unwrap();
//! let patch_json = serde_json::to_value(&patch).unwrap();
//!
//! assert_eq!(patch_json[0]["path"], "/users/[id=u-2]/name");
//! ```
mod engine;
mod error;
mod options;
mod patch_operations;
mod schema;
#[cfg(test)]
pub mod test_util;

use std::ops::{Add, Deref};

pub use options::{DiffGranularity, DiffOptions};
pub use patch_operations::PatchOp;
use serde::Serialize;

use crate::{diff::error::DiffErrorSummary, path::Spath};

/// A sequence of JSON Patch operations produced by [`diff`].
///
/// `Patch` serializes as a standard JSON Patch array. Paths may be regular JSON
/// Pointer paths, such as `/items/0/name`, or spatch semantic paths, such as
/// `/items/[id=item-42]/name`, when schema-aware diffing is enabled.
#[derive(Debug, Default, PartialEq, Eq, Serialize, Clone)]
pub struct Patch(Vec<PatchOp>);

impl Patch {
    pub fn new(operations: Vec<PatchOp>) -> Self {
        Patch(operations)
    }

    pub fn new_with_op(op: PatchOp) -> Self {
        Patch(vec![op])
    }

    pub fn push(&mut self, op: PatchOp) {
        self.0.push(op);
    }
}

impl Deref for Patch {
    type Target = Vec<PatchOp>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Add for Patch {
    type Output = Patch;

    fn add(self, rhs: Self) -> Self::Output {
        let mut new_ops = self.0;
        new_ops.extend(rhs.0);
        Patch::new(new_ops)
    }
}

impl Iterator for Patch {
    type Item = PatchOp;

    fn next(&mut self) -> Option<Self::Item> {
        if self.0.is_empty() {
            None
        } else {
            Some(self.0.remove(0))
        }
    }
}

/// Computes a patch that transforms `left` into `right`.
///
/// By default, spatch emits ordinary RFC 6902-style, index-based paths. Provide
/// a schema through [`DiffOptions::with_schema`] to generate semantic array
/// paths based on `indexKey`, and use [`DiffOptions::granular`] when you prefer
/// nested, review-friendly object changes over compact parent replacements.
///
/// Schema-aware diffing follows local JSON Schema `$ref`s in the provided schema
/// document while traversing object properties and array items. Array item
/// identity values can be strings, numbers, or booleans. Object, array, and
/// `null` identity values are rejected because they cannot be emitted as semantic
/// path filters.
///
/// # Compact by default
///
/// ```rust
/// use serde_json::json;
/// use spatch::diff::{diff, DiffOptions};
///
/// let before = json!({"name": "Ada", "city": "London"});
/// let after = json!({"name": "Ada", "city": "Oxford"});
///
/// let patch = diff(&before, &after, DiffOptions::new()).unwrap();
/// let patch_json = serde_json::to_value(&patch).unwrap();
///
/// assert_eq!(patch_json[0]["op"], "replace");
/// ```
///
/// # Schema-aware paths
///
/// ```rust
/// use serde_json::json;
/// use spatch::diff::{diff, DiffOptions};
///
/// let schema = json!({
///     "properties": {
///         "tasks": {
///             "indexKey": "id",
///             "items": { "properties": { "done": {} } }
///         }
///     }
/// });
///
/// let before = json!({"tasks": [{"id": "ship-docs", "done": false}]});
/// let after = json!({"tasks": [{"id": "ship-docs", "done": true}]});
///
/// let patch = diff(&before, &after, DiffOptions::new().with_schema(&schema).granular())
///     .unwrap();
/// let patch_json = serde_json::to_value(&patch).unwrap();
///
/// assert_eq!(patch_json[0]["path"], "/tasks/[id=ship-docs]/done");
/// ```
///
/// # Nested `$ref` schemas and numeric identity values
///
/// ```rust
/// use serde_json::json;
/// use spatch::diff::{diff, DiffOptions};
///
/// let schema = json!({
///     "properties": {
///         "levels": {
///             "indexKey": "id",
///             "items": { "$ref": "#/$defs/level" }
///         }
///     },
///     "$defs": {
///         "level": {
///             "properties": {
///                 "xp": {}
///             }
///         }
///     }
/// });
///
/// let before = json!({"levels": [{"id": 1, "xp": 100}]});
/// let after = json!({"levels": [{"id": 1, "xp": 150}]});
///
/// let patch = diff(&before, &after, DiffOptions::new().with_schema(&schema).granular())
///     .unwrap();
/// let patch_json = serde_json::to_value(&patch).unwrap();
///
/// assert_eq!(patch_json[0]["path"], "/levels/[id=1]/xp");
/// ```
pub fn diff(
    left: &serde_json::Value,
    right: &serde_json::Value,
    options: DiffOptions<'_>,
) -> Result<Patch, DiffErrorSummary> {
    let (patch, error_summary) =
        engine::diff_recursive(left, right, options, &Spath::default(), &Patch::default());

    if error_summary.is_empty() {
        Ok(patch)
    } else {
        Err(error_summary)
    }
}

#[cfg(test)]
mod tests {
    use assert2::{assert, check};
    use serde_json::json;

    use super::*;

    #[test]
    fn test_diff_simple() {
        let left = json!({
            "name": "Alice",
            "age": 30,
        });

        let right = json!({
            "name": "Alice",
            "age": 31,
        });

        let diff_options = DiffOptions::new();

        let patch = diff(&left, &right, diff_options).unwrap();
        check!(patch.len() == 1);
        assert!(let PatchOp::Replace { path, value } = &patch[0]);
        check!(path.to_string() == "/age");
        check!(value == &json!(31));
    }

    #[test]
    fn test_iterator() {
        let patch = Patch::new(vec![
            PatchOp::replace("/a".try_into().unwrap(), json!(1)),
            PatchOp::remove("/b".try_into().unwrap()),
        ]);

        for op in patch {
            match op {
                PatchOp::Replace { path, value } => {
                    check!(path.to_string() == "/a");
                    check!(value == json!(1));
                }
                PatchOp::Remove { path } => {
                    check!(path.to_string() == "/b");
                }
                _ => unreachable!(),
            }
        }
    }

    #[test]
    fn test_addition() {
        let patch1 = Patch::new(vec![PatchOp::replace("/a".try_into().unwrap(), json!(1))]);
        let patch2 = Patch::new(vec![PatchOp::remove("/b".try_into().unwrap())]);

        let combined_patch = patch1 + patch2;

        check!(combined_patch.len() == 2);
        assert!(let PatchOp::Replace { path, value } = &combined_patch[0]);
        check!(path.to_string() == "/a");
        check!(value == &json!(1));

        assert!(let PatchOp::Remove { path } = &combined_patch[1]);
        check!(path.to_string() == "/b");
    }
}
