use crate::path::Spath;

#[derive(Debug, PartialEq, Eq)]
pub enum ResolveError {
    InvalidPath,
    NotFound,
    TypeMismatch,
}

pub fn resolve(doc: &serde_json::Value, path: Spath) -> Result<&serde_json::Value, ResolveError> {
    let mut current = doc;
    for segment in path {
        current = match segment {
            crate::path::Segment::Field(field) => resolve_field(current, &field)?,
            crate::path::Segment::Filter(conditions) => resolve_filter(current, &conditions)?,
        };
    }

    Ok(current)
}

fn resolve_field<'a>(
    doc: &'a serde_json::Value,
    field: &str,
) -> Result<&'a serde_json::Value, ResolveError> {
    if !doc.is_object() && !doc.is_array() {
        return Err(ResolveError::TypeMismatch);
    }

    if doc.is_array() {
        // Try to parse field as an index
        if let Ok(index) = field.parse::<usize>() {
            let arr = doc.as_array().unwrap();
            arr.get(index).ok_or(ResolveError::NotFound)
        } else {
            Err(ResolveError::TypeMismatch)
        }
    } else {
        doc.get(field).ok_or(ResolveError::NotFound)
    }
}

type FieldName = String;
type FieldValue = String;

fn resolve_filter<'a>(
    doc: &'a serde_json::Value,
    conditions: &[(FieldName, FieldValue)],
) -> Result<&'a serde_json::Value, ResolveError> {
    let doc = doc.as_array().ok_or(ResolveError::TypeMismatch)?;

    doc.iter()
        .find(|item| {
            // Find an item that matches all conditions
            conditions.iter().all(|(filter_key, filter_value)| {
                item.get(filter_key)
                    // We only find a match if the field exists and matches the filter value
                    // is_some_and returns false if the field does not exist
                    .is_some_and(|val| value_matches_filter(val, filter_value))
            })
        })
        .ok_or(ResolveError::NotFound)
}

fn value_matches_filter(val: &serde_json::Value, filter_value: &str) -> bool {
    match val {
        serde_json::Value::String(s) => s == filter_value,
        serde_json::Value::Number(n) => value_match_number(n, filter_value),
        serde_json::Value::Bool(b) => value_match_bool(b, filter_value),
        serde_json::Value::Null => false,
        serde_json::Value::Array(_) => false,
        serde_json::Value::Object(_) => false,
    }
}

fn value_match_number(value: &serde_json::Number, filter_value: &str) -> bool {
    if value.is_f64()
        && let Ok(f) = filter_value.parse::<f64>()
    {
        return value.as_f64() == Some(f);
    }

    if value.is_u64()
        && let Ok(u) = filter_value.parse::<u64>()
    {
        return value.as_u64() == Some(u);
    }

    if value.is_i64()
        && let Ok(i) = filter_value.parse::<i64>()
    {
        return value.as_i64() == Some(i);
    }

    false
}

fn value_match_bool(value: &bool, filter_value: &str) -> bool {
    matches!((value, filter_value), (true, "true") | (false, "false"))
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
        let result = resolve(&doc, path);
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
        let result = resolve(&doc, path);
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
        let result = resolve(&doc, path).unwrap_err();

        check!(result == ResolveError::TypeMismatch);
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
        let result = resolve(&doc, path).unwrap_err();

        check!(result == ResolveError::TypeMismatch);
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
        let result = resolve(&doc, path);
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
        let result = resolve(&doc, path);
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
        let result = resolve(&doc, path);
        check!(result.is_ok());
        let result = result.unwrap();

        check!(result == &json!(1));
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
        let result = resolve(&doc, path);
        check!(result.is_ok());
        let result = result.unwrap();

        check!(result == &json!(1));
    }

    #[test]
    fn test_value_matches_filter() {
        let should_match = vec![
            (Value::String("test".to_string()), "test"),
            (Value::Bool(true), "true"),
            (Value::Bool(false), "false"),
            (Value::Number(Number::from_f64(3.001).unwrap()), "3.001"),
            (Value::Number(Number::from_f64(-3.001).unwrap()), "-3.001"),
            (Value::Number(Number::from_f64(0.0).unwrap()), "0"),
            (Value::Number(Number::from_f64(0.0).unwrap()), "0.0"),
            (Value::Number(Number::from_f64(0.0).unwrap()), "0.000"),
            (Value::Number(Number::from_u128(1).unwrap()), "1"),
            (
                Value::Number(Number::from_u128(u64::MAX as u128).unwrap()),
                "18446744073709551615",
            ),
            (Value::Number(Number::from_u128(0).unwrap()), "0"),
            (Value::Number(Number::from_u128(1).unwrap()), "01"),
            (Value::Number(Number::from_i128(1).unwrap()), "1"),
            (Value::Number(Number::from_i128(1).unwrap()), "+1"),
            (Value::Number(Number::from_i128(-1).unwrap()), "-1"),
            (Value::Number(Number::from_i128(-0).unwrap()), "0"),
            (Value::Number(Number::from_i128(0).unwrap()), "0"),
            (Value::Number(Number::from_i128(0).unwrap()), "-000"),
            (Value::Number(Number::from_i128(0).unwrap()), "00000"),
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
            (Value::Bool(true), "True"),
            (Value::Bool(true), "1"),
            (Value::Bool(true), ""),
            (Value::Bool(true), "foo"),
            (Value::Bool(false), "False"),
            (Value::Bool(false), "0"),
            (Value::Bool(false), "-1"),
            // We don't support null matching
            (Value::Null, ""),
            (Value::Null, "null"),
            (Value::Null, "0"),
            (Value::Null, "true"),
            (Value::Number(Number::from_f64(3.001).unwrap()), "3.01"),
            (Value::Number(Number::from_f64(-3.001).unwrap()), "3.001"),
            (
                Value::Number(Number::from_u128(u64::MAX as u128).unwrap()),
                "18446744073709551616",
            ),
            (Value::Number(Number::from_i128(1).unwrap()), "++1"),
            (Value::Number(Number::from_i128(-1).unwrap()), "1"),
            (Value::Number(Number::from_i128(0).unwrap()), "--000"),
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
}
