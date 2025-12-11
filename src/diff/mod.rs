mod engine;
mod patch_operations;
#[cfg(test)]
pub mod test_util;

use std::ops::Deref;

pub use patch_operations::PatchOp;

use crate::path::Spath;

#[derive(Debug, Default, PartialEq, Eq)]
pub struct Patch(Vec<PatchOp>);

impl Patch {
    pub fn new(operations: Vec<PatchOp>) -> Self {
        Patch(operations)
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

pub fn diff(
    left: &serde_json::Value,
    right: &serde_json::Value,
    schema: Option<&serde_json::Value>,
) -> Patch {
    let mut patch_ops = Patch::default();
    let mut path_pos = Spath::default();

    engine::diff_recursive(left, right, schema, &mut path_pos, &mut patch_ops);

    patch_ops
}
