mod ext;

use crate::path::{PathError, Spath};
pub use ext::SerdeValueExt;
use std::{ops::Deref, str::FromStr};

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ResolveError {
    #[error("Invalid path format")]
    InvalidPath(#[from] PathError),

    #[error("Field or item not found")]
    NotFound,

    #[error("Type mismatch encountered during resolution, expected {expected}, found {actual}")]
    TypeMismatch { expected: String, actual: String },
}

impl ResolveError {
    pub fn type_mismatch(expected: &str, found: &str) -> Self {
        ResolveError::TypeMismatch {
            expected: expected.to_string(),
            actual: found.to_string(),
        }
    }
}

pub trait ValueAccess<'a> {
    type Out: Deref<Target = serde_json::Value> + 'a;
    type ArrayIter: Iterator<Item = Self::Out> + 'a;

    fn is_object(&self) -> bool;
    fn is_array(&self) -> bool;

    fn get_key(self, key: &str) -> Option<Self::Out>;
    fn get_index(self, index: usize) -> Option<Self::Out>;

    fn array_iter(self) -> Option<Self::ArrayIter>;
}

impl<'a> ValueAccess<'a> for &'a serde_json::Value {
    type Out = &'a serde_json::Value;
    type ArrayIter = std::slice::Iter<'a, serde_json::Value>;

    fn is_object(&self) -> bool {
        serde_json::Value::is_object(self)
    }
    fn is_array(&self) -> bool {
        serde_json::Value::is_array(self)
    }
    fn get_key(self, key: &str) -> Option<Self::Out> {
        self.get(key)
    }
    fn get_index(self, index: usize) -> Option<Self::Out> {
        self.get(index)
    }
    fn array_iter(self) -> Option<Self::ArrayIter> {
        self.as_array().map(|v| v.iter())
    }
}

impl<'a> ValueAccess<'a> for &'a mut serde_json::Value {
    type Out = &'a mut serde_json::Value;
    type ArrayIter = std::slice::IterMut<'a, serde_json::Value>;

    fn is_object(&self) -> bool {
        serde_json::Value::is_object(self)
    }
    fn is_array(&self) -> bool {
        serde_json::Value::is_array(self)
    }
    fn get_key(self, key: &str) -> Option<Self::Out> {
        self.get_mut(key)
    }
    fn get_index(self, index: usize) -> Option<Self::Out> {
        self.get_mut(index)
    }
    fn array_iter(self) -> Option<Self::ArrayIter> {
        self.as_array_mut().map(|v| v.iter_mut())
    }
}

pub fn resolve_ref<'a>(
    doc: &'a serde_json::Value,
    path: &Spath,
) -> Result<&'a serde_json::Value, ResolveError> {
    resolve_inner(doc, path)
}

pub fn resolve_mut<'a>(
    doc: &'a mut serde_json::Value,
    path: &'a Spath,
) -> Result<&'a mut serde_json::Value, ResolveError> {
    resolve_inner(doc, path)
}

fn resolve_inner<'a, 'b, A>(doc: A, path: &'b Spath) -> Result<A::Out, ResolveError>
where
    A: ValueAccess<'a, Out = A>, // output type is the same as input type
    A: std::ops::Deref<Target = serde_json::Value>,
{
    let mut current: A::Out = doc;
    for segment in path {
        current = match segment {
            crate::path::Segment::Field(field) => resolve_field(current, field)?,
            crate::path::Segment::Filter(conditions) => resolve_filter(current, conditions)?,
        };
    }

    Ok(current)
}

fn resolve_field<'a, A>(doc: A, field: &str) -> Result<A::Out, ResolveError>
where
    A: ValueAccess<'a>,
    A: Deref<Target = serde_json::Value>,
{
    let type_name = value_type_desc(&doc);
    if !doc.is_object() && !doc.is_array() {
        return Err(ResolveError::type_mismatch("object or array", &type_name));
    }

    if doc.is_array() {
        // Try to parse field as an index
        if let Ok(index) = field.parse::<usize>() {
            doc.get_index(index).ok_or(ResolveError::NotFound)
        } else {
            Err(ResolveError::type_mismatch(
                "number",
                &format!("string({field:?})"),
            ))
        }
    } else {
        doc.get_key(field).ok_or(ResolveError::NotFound)
    }
}

type FieldName = String;
type FieldValue = String;

fn resolve_filter<'a, A>(
    doc: A,
    conditions: &[(FieldName, FieldValue)],
) -> Result<A::Out, ResolveError>
where
    A: ValueAccess<'a>,
    A: Deref<Target = serde_json::Value>,
{
    let type_name = value_type_desc(&doc);
    let arr = doc
        .array_iter()
        .ok_or(ResolveError::type_mismatch("array", &type_name))?;

    arr.into_iter()
        .find_map(|item| {
            // Find an item that matches all conditions
            let matches = conditions.iter().all(|(k, v)| {
                item.deref()
                    .get(k) // use shared view for matching
                    .is_some_and(|val| value_matches_filter(val, v))
            });

            // If matches, return ref to item
            matches.then_some(item)
        })
        .ok_or(ResolveError::NotFound)
}

fn value_matches_filter<V>(val: V, filter_value: &str) -> bool
where
    V: Deref<Target = serde_json::Value>,
{
    match val.deref() {
        serde_json::Value::String(s) => s == filter_value,
        serde_json::Value::Number(n) => value_match_number(n, filter_value),
        serde_json::Value::Bool(b) => value_match_bool(b, filter_value),
        serde_json::Value::Null => false,
        serde_json::Value::Array(_) => false,
        serde_json::Value::Object(_) => false,
    }
}

fn value_match_number(value: &serde_json::Number, filter_value: &str) -> bool {
    match serde_json::Number::from_str(filter_value) {
        Ok(parsed) => &parsed == value,
        Err(_) => false,
    }
}

fn value_match_bool(value: &bool, filter_value: &str) -> bool {
    matches!(
        (value, (filter_value.to_lowercase()).as_ref()),
        (true, "true") | (false, "false")
    )
}

fn value_type_desc(val: &serde_json::Value) -> String {
    match val {
        serde_json::Value::Null => "null".to_string(),
        serde_json::Value::Bool(b) => format!("boolean({b})"),
        serde_json::Value::Number(n) => format!("number({n})"),
        serde_json::Value::String(s) => format!("string({s:?})"),
        serde_json::Value::Array(_) => "array".to_string(),
        serde_json::Value::Object(_) => "object".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use assert2::check;
    use serde_json::{Number, Value, json};

    use crate::path::Segment;

    use super::*;

    #[test]
    fn test_resolve_simple_path() {
        let doc = json!({
            "a": {
                "b": {
                    "c": 42
                }
            }
        });
        let path = Spath {
            segments: vec![
                Segment::Field("a".to_string()),
                Segment::Field("b".to_string()),
                Segment::Field("c".to_string()),
            ],
        };
        let result = resolve_inner(&doc, &path);
        check!(result.is_ok());
        let result = result.unwrap();
        check!(result == &json!(42));
    }

    #[test]
    fn test_resolve_field_not_found() {
        let doc = json!({
            "a": {
                "b": {
                    "c": 42
                }
            }
        });
        let path = Spath {
            segments: vec![
                Segment::Field("a".to_string()),
                Segment::Field("x".to_string()), // 'x' does not exist
            ],
        };
        let result = resolve_inner(&doc, &path);
        check!(matches!(result, Err(ResolveError::NotFound)));
    }

    #[test]
    fn test_resolve_type_mismatch() {
        let doc = json!({
            "a": {
                "b": 42
            }
        });
        let path = Spath {
            segments: vec![
                Segment::Field("a".to_string()),
                Segment::Field("b".to_string()),
                Segment::Field("c".to_string()), // 'b' is not an object
            ],
        };
        let result = resolve_inner(&doc, &path).unwrap_err();

        check!(result == ResolveError::type_mismatch("object or array", "number(42)"));
    }

    #[test]
    fn test_resolve_filter_type_mismatch() {
        let doc = json!({
            "a": {
                "b": 42
            }
        });
        let path = Spath {
            segments: vec![
                Segment::Field("a".to_string()),
                Segment::Filter(vec![("id".to_string(), "foo".to_string())]),
            ],
        };
        let result = resolve_inner(&doc, &path).unwrap_err();

        check!(result == ResolveError::type_mismatch("array", "object"));
    }

    #[test]
    fn test_resolve_filter() {
        let doc = json!({
            "items": [
                { "id": "foo", "value": 1 },
                { "id": "bar", "value": 2 }
            ]
        });
        let path = Spath {
            segments: vec![
                Segment::Field("items".to_string()),
                Segment::Filter(vec![("id".to_string(), "foo".to_string())]),
            ],
        };
        let result = resolve_inner(&doc, &path);
        check!(result.is_ok());
        let result = result.unwrap();

        check!(result == &json!({"id": "foo", "value": 1}));
    }

    #[test]
    fn test_resolve_filter_should_return_inner_value() {
        let doc = json!({
            "items": [
                { "id": "foo", "value": 1 },
                { "id": "bar", "value": 2 }
            ]
        });
        let path = Spath {
            segments: vec![
                Segment::Field("items".to_string()),
                Segment::Filter(vec![("id".to_string(), "foo".to_string())]),
                Segment::Field("value".to_string()),
            ],
        };
        let result = resolve_inner(&doc, &path);
        check!(result.is_ok());
        let result = result.unwrap();

        check!(result == &json!(1));
    }

    #[test]
    fn test_resolve_filter_with_multiple_filters_should_return_inner_value() {
        let doc = json!({
            "items": [
                { "id": "foo", "isActive": true, "value": 1 },
                { "id": "bar", "isActive": false, "value": 2 }
            ]
        });
        let path = Spath {
            segments: vec![
                Segment::Field("items".to_string()),
                Segment::Filter(vec![
                    ("id".to_string(), "foo".to_string()),
                    ("isActive".to_string(), "true".to_string()),
                ]),
                Segment::Field("value".to_string()),
            ],
        };
        let result = resolve_inner(&doc, &path);
        check!(result.is_ok());
        let result = result.unwrap();

        check!(result == &json!(1));
    }

    #[test]
    fn test_resolve_filter_with_multiple_filters_and_matching_false_should_return_inner_value() {
        let doc = json!({
            "items": [
                { "id": "foo", "isActive": true, "value": 1 },
                { "id": "foo", "isActive": false, "value": 3 },
                { "id": "bar", "isActive": false, "value": 2 }
            ]
        });
        let path = Spath {
            segments: vec![
                Segment::Field("items".to_string()),
                Segment::Filter(vec![
                    ("id".to_string(), "foo".to_string()),
                    ("isActive".to_string(), "false".to_string()),
                ]),
                Segment::Field("value".to_string()),
            ],
        };
        let result = resolve_inner(&doc, &path);
        check!(result.is_ok());
        let result = result.unwrap();

        check!(result == &json!(3));
    }

    #[test]
    fn test_resolve_with_field_segment_should_return_array_item_by_index() {
        let doc = json!({
            "items": [
                { "id": "foo", "value": 1 },
                { "id": "bar", "value": 2 }
            ]
        });
        let path = Spath {
            segments: vec![
                Segment::Field("items".to_string()),
                Segment::Field("0".to_string()),
                Segment::Field("value".to_string()),
            ],
        };
        let result = resolve_inner(&doc, &path);
        check!(result.is_ok());
        let result = result.unwrap();

        check!(result == &json!(1));
    }

    #[test]
    fn test_resolve_with_field_segment_not_matching_type_should_return_a_type_mismatch() {
        let doc = json!({
            "items": [
                { "id": "foo", "value": 1 },
                { "id": "bar", "value": 2 }
            ]
        });
        let path = Spath {
            segments: vec![
                Segment::Field("items".to_string()),
                Segment::Field("foo".to_string()),
                Segment::Field("value".to_string()),
            ],
        };
        let result = resolve_inner(&doc, &path);

        check!(result == Err(ResolveError::type_mismatch("number", "string(\"foo\")")));
    }

    #[test]
    fn test_value_matches_filter() {
        let should_match = vec![
            (Value::String("test".to_string()), "test"),
            (Value::Bool(true), "true"),
            (Value::Bool(true), "True"),
            (Value::Bool(false), "false"),
            (Value::Bool(false), "False"),
            (Value::Number(Number::from_f64(3.001).unwrap()), "3.001"),
            (Value::Number(Number::from_f64(-3.001).unwrap()), "-3.001"),
            (Value::Number(Number::from_f64(0.0).unwrap()), "0.0"),
            (Value::Number(Number::from_f64(0.0).unwrap()), "0.000"),
            (Value::Number(Number::from_u128(1).unwrap()), "1"),
            (
                Value::Number(Number::from_u128(u64::MAX as u128).unwrap()),
                "18446744073709551615",
            ),
            (Value::Number(Number::from_u128(0).unwrap()), "0"),
            (Value::Number(Number::from_i128(1).unwrap()), "1"),
            (Value::Number(Number::from_i128(-1).unwrap()), "-1"),
            (Value::Number(Number::from_i128(-0).unwrap()), "0"),
            (Value::Number(Number::from_i128(0).unwrap()), "0"),
            (
                Value::Number(Number::from_i128(i64::MAX as i128).unwrap()),
                "9223372036854775807",
            ),
            (
                Value::Number(Number::from_i128(i64::MIN as i128).unwrap()),
                "-9223372036854775808",
            ),
        ];

        let should_not_match = vec![
            (Value::String("test".to_string()), "Test"),
            (Value::String("test".to_string()), " test "),
            (Value::Bool(true), "1"),
            (Value::Bool(true), ""),
            (Value::Bool(true), "foo"),
            (Value::Bool(false), "0"),
            (Value::Bool(false), "-1"),
            // We don't support null matching
            (Value::Null, ""),
            (Value::Null, "null"),
            (Value::Null, "0"),
            (Value::Null, "true"),
            (Value::Number(Number::from_f64(3.001).unwrap()), "3.01"),
            (Value::Number(Number::from_f64(-3.001).unwrap()), "3.001"),
            (Value::Number(Number::from_f64(0.0).unwrap()), "0"),
            (Value::Number(Number::from_u128(1).unwrap()), "01"),
            (
                Value::Number(Number::from_u128(u64::MAX as u128).unwrap()),
                "18446744073709551616",
            ),
            (Value::Number(Number::from_i128(1).unwrap()), "+1"),
            (Value::Number(Number::from_i128(1).unwrap()), "++1"),
            (Value::Number(Number::from_i128(-1).unwrap()), "1"),
            (Value::Number(Number::from_i128(0).unwrap()), "--000"),
            (Value::Number(Number::from_i128(0).unwrap()), "-000"),
            (Value::Number(Number::from_i128(0).unwrap()), "00000"),
            (
                Value::Number(Number::from_i128(i64::MAX as i128).unwrap()),
                "9223372036854775808",
            ),
            (
                Value::Number(Number::from_i128(i64::MIN as i128).unwrap()),
                "9223372036854775808",
            ),
            (Value::Array(vec![]), ""),
            (Value::Array(vec![Value::Number(Number::from(1))]), "1"),
            (Value::Object(serde_json::Map::new()), ""),
        ];

        for (number, filter_str) in should_match {
            check!(
                value_matches_filter(&number, filter_str),
                "{:?} did not match {:?}",
                &number,
                filter_str
            );
        }

        for (number, filter_str) in should_not_match {
            check!(
                !value_matches_filter(&number, filter_str),
                "{:?} matched {:?}",
                &number,
                filter_str
            );
        }
    }

    #[test]
    fn value_match_bool_test() {
        check!(value_match_bool(&true, "true"));
        check!(value_match_bool(&false, "false"));

        check!(!value_match_bool(&false, "true"));
        check!(!value_match_bool(&true, "false"));

        check!(!value_match_bool(&true, "1"));
        check!(!value_match_bool(&true, "0"));
        check!(!value_match_bool(&true, "-1"));
        check!(!value_match_bool(&true, "foo"));
        check!(!value_match_bool(&true, ""));

        check!(!value_match_bool(&false, "1"));
        check!(!value_match_bool(&false, "0"));
        check!(!value_match_bool(&false, "-1"));
        check!(!value_match_bool(&false, "foo"));
        check!(!value_match_bool(&false, ""));
    }

    #[test]
    fn resolve_filter_should_return_mutable_value() {
        let mut doc = json!({
            "items": [
                { "id": "foo", "value": 1 },
                { "id": "bar", "value": 2 }
            ]
        });
        let expected = json!({"id": "foo", "value": 42});
        let path = Spath {
            segments: vec![
                Segment::Field("items".to_string()),
                Segment::Filter(vec![("id".to_string(), "foo".to_string())]),
            ],
        };
        let result = resolve_inner(&mut doc, &path);
        check!(result.is_ok());
        let result = result.unwrap();
        result["value"] = json!(42);

        check!(result == &expected);
        check!(doc["items"][0] == expected);
    }

    #[test]
    fn resolve_field_should_return_mutable_value() {
        let mut doc = json!({
            "items": [
                { "id": "foo", "value": 1 },
                { "id": "bar", "value": 2 }
            ]
        });
        let expected = json!({"id": "foo", "value": 42});
        let path = Spath {
            segments: vec![
                Segment::Field("items".to_string()),
                Segment::Field("0".to_string()),
            ],
        };
        let result = resolve_inner(&mut doc, &path);
        check!(result.is_ok());
        let result = result.unwrap();
        result["value"] = json!(42);

        check!(result == &expected);
        check!(doc["items"][0] == expected);
    }
}
