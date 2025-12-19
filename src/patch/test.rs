use serde_json::Value;

use crate::{patch::error::PatchError, path::Spath, resolve::resolve_ref};

/// The "test" operation tests that a value at the target location is
/// equal to a specified value.
///
/// The operation object MUST contain a "value" member that conveys the
/// value to be compared to the target location's value.
///
/// The target location MUST be equal to the "value" value for the
/// operation to be considered successful.
///
/// Here, "equal" means that the value at the target location and the
/// value conveyed by "value" are of the same JSON type, and that they
/// are considered equal by the following rules for that type:
///
/// o  strings: are considered equal if they contain the same number of
///     Unicode characters and their code points are byte-by-byte equal.
///
/// o  numbers: are considered equal if their values are numerically
///     equal.
///
/// o  arrays: are considered equal if they contain the same number of
///     values, and if each value can be considered equal to the value at
///     the corresponding position in the other array, using this list of
///     type-specific rules.
///
/// o  objects: are considered equal if they contain the same number of
///     members, and if each member can be considered equal to a member in
///     the other object, by comparing their keys (as strings) and their
///     values (using this list of type-specific rules).
///
/// o  literals (false, true, and null): are considered equal if they are
///     the same.
///
/// Note that the comparison that is done is a logical comparison; e.g.,
/// whitespace between the member values of an array is not significant.
///
/// Also, note that ordering of the serialization of object members is
/// not significant.
///
/// For example:
///
/// { "op": "test", "path": "/a/b/c", "value": "foo" }
pub fn test(doc: &mut Value, path: Spath, value: Value) -> Result<(), PatchError> {
    let value_at_path = resolve_ref(doc, &path)?;

    if value_at_path != &value {
        return Err(PatchError::ValuesNotEqual);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use assert2::{check, let_assert};
    use serde_json::json;

    use crate::resolve::ResolveError;

    use super::*;

    #[test]
    fn test_should_succeed_for_equal_values() {
        let mut doc = json!({"a": 1, "b": 2});

        let_assert!(Ok(_) = test(&mut doc, "/a".try_into().unwrap(), json!(1)));
        let_assert!(Ok(_) = test(&mut doc, "/b".try_into().unwrap(), json!(2)));
        check!(doc == json!({"a": 1, "b": 2}));
    }

    #[test]
    fn test_should_fail_for_unequal_values() {
        let mut doc = json!({"a": 1, "b": 2});

        let_assert!(
            Err(PatchError::ValuesNotEqual) = test(&mut doc, "/a".try_into().unwrap(), json!(42))
        );
        let_assert!(
            Err(PatchError::ValuesNotEqual) = test(&mut doc, "/b".try_into().unwrap(), json!(3))
        );
        check!(doc == json!({"a": 1, "b": 2}));
    }

    #[test]
    fn test_should_fail_for_nonexistent_path() {
        let mut doc = json!({"a": 1, "b": 2});

        let_assert!(
            Err(PatchError::ResolveError(ResolveError::NotFound)) =
                test(&mut doc, "/c".try_into().unwrap(), json!(3))
        );
        check!(doc == json!({"a": 1, "b": 2}));
    }

    #[test]
    fn test_should_succeed_for_complex_equal_values() {
        let mut doc = json!({"a": {"b": [1, 2, 3], "c": "hello"}, "d": true});

        let_assert!(
            Ok(_) = test(
                &mut doc,
                "/a".try_into().unwrap(),
                json!({"b": [1, 2, 3], "c": "hello"})
            )
        );
        let_assert!(Ok(_) = test(&mut doc, "/a/b".try_into().unwrap(), json!([1, 2, 3])));
        let_assert!(Ok(_) = test(&mut doc, "/a/c".try_into().unwrap(), json!("hello")));
        let_assert!(Ok(_) = test(&mut doc, "/d".try_into().unwrap(), json!(true)));
        check!(doc == json!({"a": {"b": [1, 2, 3], "c": "hello"}, "d": true}));
    }
}
