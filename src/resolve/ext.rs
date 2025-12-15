use crate::path::Spath;

use super::{ResolveError, resolve};

pub trait SerdeValueExt {
    fn get_value_at(&self, path: &str) -> Result<&serde_json::Value, ResolveError>;
    fn apply_at(&self, path: &str, value: serde_json::Value) -> Result<serde_json::Value, ResolveError>;
}

impl SerdeValueExt for serde_json::Value {
    fn get_value_at(&self, path: &str) -> Result<&serde_json::Value, ResolveError> {
        let spath = Spath::try_from(path)?;

        resolve(self, spath)
    }
    fn apply_at(&self, path: &str, value: serde_json::Value) -> Result<serde_json::Value, ResolveError> {
        todo!()
    }
}
