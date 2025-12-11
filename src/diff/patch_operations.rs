use crate::path::Spath;

#[derive(Debug, PartialEq, Eq)]
pub enum PatchOp {
    Add {
        path: Spath,
        value: serde_json::Value,
    },
    Remove {
        path: Spath,
    },
    Replace {
        path: Spath,
        value: serde_json::Value,
    },
    Move {
        from: Spath,
        path: Spath,
    },
    // Copy {
    //     from: Spath,
    //     path: Spath,
    // },
    Test {
        path: Spath,
        value: serde_json::Value,
    },
}

impl PatchOp {
    pub fn replace(path: Spath, value: serde_json::Value) -> Self {
        PatchOp::Replace { path, value }
    }

    pub fn remove(path: Spath) -> Self {
        PatchOp::Remove { path }
    }

    pub fn add(path: Spath, value: serde_json::Value) -> Self {
        PatchOp::Add { path, value }
    }

    pub fn move_op(from: Spath, path: Spath) -> Self {
        PatchOp::Move { from, path }
    }
}
