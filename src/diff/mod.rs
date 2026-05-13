mod engine;
mod error;
mod patch_operations;
#[cfg(test)]
pub mod test_util;

use std::ops::{Add, Deref};

pub use patch_operations::PatchOp;
use serde::Serialize;

use crate::{diff::error::DiffErrorSummary, path::Spath};

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

pub fn diff(
    left: &serde_json::Value,
    right: &serde_json::Value,
    schema: Option<&serde_json::Value>,
) -> Result<Patch, DiffErrorSummary> {
    let (patch, error_summary) =
        engine::diff_recursive(left, right, schema, &Spath::default(), &Patch::default());

    if error_summary.is_empty() {
        Ok(patch)
    } else {
        Err(error_summary)
    }
}

#[cfg(test)]
mod tests {
    use assert2::{check, let_assert};
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

        let patch = diff(&left, &right, None).unwrap();
        check!(patch.len() == 1);
        let_assert!(PatchOp::Replace { path, value } = &patch[0]);
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
        let_assert!(PatchOp::Replace { path, value } = &combined_patch[0]);
        check!(path.to_string() == "/a");
        check!(value == &json!(1));

        let_assert!(PatchOp::Remove { path } = &combined_patch[1]);
        check!(path.to_string() == "/b");
    }
}
