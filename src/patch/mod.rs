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
    for op in patch {
        match op {
            PatchOp::Add { path, value } => add(&mut doc, path.clone(), value.clone())?,
            PatchOp::Remove { path } => remove(&mut doc, path.clone())?,
            PatchOp::Replace { path, value } => replace(&mut doc, path.clone(), value.clone())?,
            PatchOp::Move { from, path } => move_op(&mut doc, from.clone(), path.clone())?,
            PatchOp::Copy { from, path } => copy(&mut doc, from.clone(), path.clone())?,
            PatchOp::Test { path, value } => test(&mut doc, path.clone(), value.clone())?,
        }
    }
    Ok(doc)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diff::{
        PatchOp,
        test_util::json_patch_tests::{self, TestScope, TestVariant},
    };

    fn map_op(op: &serde_json::Value) -> PatchOp {
        match op.get("op").and_then(|v| v.as_str()) {
            Some("add") => PatchOp::Add {
                path: op
                    .get("path")
                    .unwrap()
                    .as_str()
                    .unwrap()
                    .try_into()
                    .unwrap(),
                value: op.get("value").unwrap().clone(),
            },
            Some("remove") => PatchOp::Remove {
                path: op
                    .get("path")
                    .unwrap()
                    .as_str()
                    .unwrap()
                    .try_into()
                    .unwrap(),
            },
            Some("replace") => PatchOp::Replace {
                path: op
                    .get("path")
                    .unwrap()
                    .as_str()
                    .unwrap()
                    .try_into()
                    .unwrap(),
                value: op.get("value").unwrap().clone(),
            },
            Some("move") => PatchOp::Move {
                from: op
                    .get("from")
                    .unwrap()
                    .as_str()
                    .unwrap()
                    .try_into()
                    .unwrap(),
                path: op
                    .get("path")
                    .unwrap()
                    .as_str()
                    .unwrap()
                    .try_into()
                    .unwrap(),
            },
            Some("copy") => PatchOp::Copy {
                from: op
                    .get("from")
                    .unwrap()
                    .as_str()
                    .unwrap()
                    .try_into()
                    .unwrap(),
                path: op
                    .get("path")
                    .unwrap()
                    .as_str()
                    .unwrap()
                    .try_into()
                    .unwrap(),
            },
            Some("test") => PatchOp::Test {
                path: op
                    .get("path")
                    .unwrap()
                    .as_str()
                    .unwrap()
                    .try_into()
                    .unwrap(),
                value: op.get("value").unwrap().clone(),
            },
            // Some("remove") => PatchOp::Remove {
            _ => todo!(),
        }
    }
    #[test]
    fn applying_patch_with_failing_test_should_not_apply_any_changes() {
        // TODO: Implement this test based on RFC 6902 Section 5
        // https://datatracker.ietf.org/doc/html/rfc6902#section-5
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

                    let patches: Vec<PatchOp> = patch.iter().map(|op| map_op(op)).collect();

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
