use crate::path::Spath;

#[derive(Debug, PartialEq, Eq, Clone)]
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
    // Move {
    //     from: Spath,
    //     path: Spath,
    // },
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

    // pub fn move_op(from: Spath, path: Spath) -> Self {
    //     PatchOp::Move { from, path }
    // }
}

impl serde::Serialize for PatchOp {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeMap;

        let mut map = serializer.serialize_map(None)?;

        match self {
            PatchOp::Add { path, value } => {
                map.serialize_entry("op", "add")?;
                map.serialize_entry("path", &path)?;
                map.serialize_entry("value", value)?;
            }
            PatchOp::Remove { path } => {
                map.serialize_entry("op", "remove")?;
                map.serialize_entry("path", &path)?;
            }
            PatchOp::Replace { path, value } => {
                map.serialize_entry("op", "replace")?;
                map.serialize_entry("path", &path)?;
                map.serialize_entry("value", value)?;
            }
            PatchOp::Test { path, value } => {
                map.serialize_entry("op", "test")?;
                map.serialize_entry("path", &path)?;
                map.serialize_entry("value", value)?;
            }
        }

        map.end()
    }
}
