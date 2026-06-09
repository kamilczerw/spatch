use std::collections::HashSet;

use serde_json::Value;

const MAX_REF_DEPTH: usize = 64;

#[derive(Debug, Clone, Copy)]
pub struct SchemaResolver<'a> {
    root: Option<&'a Value>,
}

impl<'a> SchemaResolver<'a> {
    pub fn new(root: Option<&'a Value>) -> Self {
        Self { root }
    }

    pub(super) fn resolve(&self, schema: &'a Value) -> Option<&'a Value> {
        self.resolve_inner(schema, &mut HashSet::new(), 0)
    }

    pub fn property_schema(&self, schema: Option<&'a Value>, key: &str) -> Option<&'a Value> {
        let schema = self.resolve(schema?)?;
        let property = schema.get("properties")?.get(key)?;
        self.resolve(property)
    }

    pub fn items_schema(&self, schema: Option<&'a Value>) -> Option<&'a Value> {
        let schema = self.resolve(schema?)?;
        let items = schema.get("items")?;
        self.resolve(items)
    }

    pub fn index_key(&self, schema: Option<&'a Value>) -> Option<&'a str> {
        let schema = self.resolve(schema?)?;
        schema.get(super::engine::HASH_KEY_PROP_NAME)?.as_str()
    }

    fn resolve_inner(
        &self,
        schema: &'a Value,
        visited: &mut HashSet<String>,
        depth: usize,
    ) -> Option<&'a Value> {
        if depth >= MAX_REF_DEPTH {
            return Some(schema);
        }

        let Some(reference) = schema.get("$ref").and_then(Value::as_str) else {
            return Some(schema);
        };

        if !visited.insert(reference.to_owned()) {
            return Some(schema);
        }

        let target = self.resolve_local_ref(reference)?;
        self.resolve_inner(target, visited, depth + 1)
    }

    fn resolve_local_ref(&self, reference: &str) -> Option<&'a Value> {
        let pointer = reference.strip_prefix('#')?;
        self.root?.pointer(pointer)
    }
}
