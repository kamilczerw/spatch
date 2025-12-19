mod add;
mod copy;
mod error;
mod move_op;
mod remove;
mod replace;
mod test;

pub use add::add;
pub use copy::copy;
pub use move_op::move_op;
pub use remove::remove;
pub use replace::replace;
use serde_json::Value;
pub use test::test;

use crate::{diff::PatchOp, patch::error::PatchError};

pub fn apply(doc: &Value, patch: &[PatchOp]) -> Result<Value, PatchError> {
    let mut doc = doc.clone();
    let mut failures = Vec::new();
    for op in patch {
        let result = match op {
            PatchOp::Add { path, value } => add(&mut doc, path.clone(), value.clone()),
            PatchOp::Remove { path } => remove(&mut doc, path.clone()),
            PatchOp::Replace { path, value } => replace(&mut doc, path.clone(), value.clone()),
            PatchOp::Move { from, path } => move_op(&mut doc, from.clone(), path.clone()),
            PatchOp::Copy { from, path } => copy(&mut doc, from.clone(), path.clone()),
            PatchOp::Test { path, value } => test(&mut doc, path.clone(), value.clone()),
        };

        match result {
            Ok(_) => {}
            Err(e) => failures.push(e),
        }
    }

    if !failures.is_empty() {
        Err(PatchError::MultipleErrors(failures))
    } else {
        Ok(doc)
    }
}

#[cfg(test)]
mod tests {
    use assert2::{check, let_assert};
    use serde_json::json;

    use super::*;
    use crate::diff::{
        PatchOp,
        test_util::json_patch_tests::{self, TestScope, TestVariant},
    };

    fn map_op(op: &serde_json::Value) -> PatchOp {
        serde_json::from_value::<PatchOp>(op.clone()).unwrap()
    }

    #[test]
    fn apply_with_failing_test_should_not_apply_any_changes() {
        let doc = serde_json::json!({
            "a": 1,
            "b": 2
        });
        let patches = vec![
            PatchOp::add("/c".try_into().unwrap(), json!({"foo": "bar"})),
            PatchOp::test("/a".try_into().unwrap(), json!(2)), // This test will fail
        ];

        let_assert!(Err(PatchError::MultipleErrors(errors)) = apply(&doc, &patches));

        check!(errors.len() == 1);
    }

    #[test]
    fn apply_with_all_successful_operations_should_apply_all_changes() {
        let doc = serde_json::json!({
            "a": 1,
            "b": 2
        });
        let patches = vec![
            PatchOp::add("/c".try_into().unwrap(), json!({"foo": "bar"})),
            PatchOp::test("/a".try_into().unwrap(), json!(1)), // This test will pass
            PatchOp::replace("/b".try_into().unwrap(), json!({"baz": [1, 2, 3]})),
        ];

        let_assert!(Ok(resulting_doc) = apply(&doc, &patches));

        check!(
            resulting_doc
                == json!({
                    "a": 1,
                    "b": {"baz": [1, 2, 3]},
                    "c": {"foo": "bar"}
                })
        );
    }

    #[test]
    fn test_apply_against_the_jsonpatch_spec_tests() {
        let mut test_cases =
            json_patch_tests::load_json_patch_test_cases(TestScope::Patch, TestVariant::Tests);
        test_cases.extend(json_patch_tests::load_json_patch_test_cases(
            TestScope::Patch,
            TestVariant::SpecTests,
        ));

        // Collect all failures instead of failing immediately.
        let mut failures = Vec::new();

        for test_case in test_cases {
            match test_case {
                json_patch_tests::JsonPatchTestCase::Valid {
                    doc,
                    patch,
                    expected,
                    comment,
                } => {
                    let comment = comment.unwrap_or_default();

                    let patches: Vec<PatchOp> = patch.iter().map(map_op).collect();

                    let result = apply(&doc, &patches);

                    if let Err(e) = result {
                        failures.push(format!(
                            "Failed test case: {comment}\n  Error applying patch: {error}",
                            comment = comment,
                            error = e,
                        ));
                        continue;
                    }

                    if let Ok(resulting_doc) = result
                        && resulting_doc != expected
                    {
                        failures.push(format!(
                            "Failed test case: {comment}\n  Resulting document does not match expected.\n  Resulting: {resulting:?}\n  Expected: {expected:?}",
                            comment = comment,
                            resulting = resulting_doc,
                            expected = expected,
                        ));
                    }
                }
                json_patch_tests::JsonPatchTestCase::Failure { .. } => {
                    // TODO: Implement failure test cases
                }
            }
        }

        // Only fail once, after we've run all cases.
        if !failures.is_empty() {
            panic!(
                "jsonpatch spec compliance failed for {} case(s):\n\n{}\n\nNumber of failures: {}",
                failures.len(),
                failures.join("\n\n"),
                failures.len(),
            );
        }
    }
}
