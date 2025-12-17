use crate::{path::Spath, resolve::resolve_mut};

use super::{ResolveError, resolve_ref};

pub trait SerdeValueExt {
    fn get_value_at(&self, path: &str) -> Result<&serde_json::Value, ResolveError>;
    fn apply_at(&mut self, path: &str, value: serde_json::Value) -> Result<(), ResolveError>;
}

impl SerdeValueExt for serde_json::Value {
    fn get_value_at(&self, path: &str) -> Result<&serde_json::Value, ResolveError> {
        let spath = Spath::try_from(path)?;

        resolve_ref(self, &spath)
    }

    fn apply_at(&mut self, path: &str, value: serde_json::Value) -> Result<(), ResolveError> {
        let spath = Spath::try_from(path)?;

        let doc = resolve_mut(self, &spath)?;
        *doc = value;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use assert2::check;

    use crate::path::PathError;

    use super::*;

    #[test]
    fn get_value_at_should_return_value_at_path() {
        let json: serde_json::Value = serde_json::from_str(
            r#"
        {
            "a": {
                "b": [1, 2, 3],
                "c": "hello"
            },
            "d": true
        }
        "#,
        )
        .unwrap();

        let value = json.get_value_at("/a/b/1").unwrap();
        assert_eq!(value, &serde_json::Value::from(2));

        let value = json.get_value_at("/a/c").unwrap();
        assert_eq!(value, &serde_json::Value::from("hello"));

        let value = json.get_value_at("/d").unwrap();
        assert_eq!(value, &serde_json::Value::from(true));
    }

    #[test]
    fn get_value_at_should_return_error_for_invalid_path() {
        let json: serde_json::Value = serde_json::from_str(
            r#"
        {
            "a": {
                "b": [1, 2, 3],
                "c": "hello"
            },
            "d": true
        }
        "#,
        )
        .unwrap();

        let err = json.get_value_at("/a/b/10").unwrap_err();
        check!(err == ResolveError::NotFound);

        let err = json.get_value_at("/a/x").unwrap_err();
        check!(err == ResolveError::NotFound);

        let err = json.get_value_at("invalid_path").unwrap_err();
        check!(
            err == ResolveError::InvalidPath(PathError::invalid_syntax(
                0,
                "expected a path starting with '/' or empty input"
            ))
        );
    }

    #[test]
    fn apply_at_should_modify_the_value_at_the_specified_path() {
        let mut json: serde_json::Value = serde_json::from_str(
            r#"
        {
            "a": {
                "b": [1, 2, 3],
                "c": "hello"
            },
            "d": true
        }
        "#,
        )
        .unwrap();

        json.apply_at("/a/c", serde_json::Value::from("world"))
            .unwrap();

        check!(json.get_value_at("/a/c").unwrap() == &serde_json::Value::from("world"));
    }

    #[test]
    fn apply_at_should_modify_the_value_at_the_specified_semantic_path() {
        let mut json: serde_json::Value = serde_json::from_str(
            r#"
        {
            "list": [
                {"id": "item1", "value": "hello"},
                {"id": "item2", "value": "world"}
            ]
        }
        "#,
        )
        .unwrap();

        json.apply_at("/list/[id=item2]/value", serde_json::Value::from("foo"))
            .unwrap();

        check!(
            json.get_value_at("/list/[id=item2]/value").unwrap() == &serde_json::Value::from("foo")
        );
    }

    #[test]
    fn apply_at_should_modify_the_value_at_the_specified_index_path() {
        let mut json: serde_json::Value = serde_json::from_str(
            r#"
        {
            "list": [
                {"id": "item1", "value": "hello"},
                {"id": "item2", "value": "world"}
            ]
        }
        "#,
        )
        .unwrap();

        json.apply_at("/list/0/value", serde_json::Value::from("foo"))
            .unwrap();

        check!(
            json.get_value_at("/list/[id=item1]/value").unwrap() == &serde_json::Value::from("foo")
        );
    }
}
