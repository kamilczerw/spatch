pub enum JsonPatchTestCase {
    Valid {
        doc: serde_json::Value,
        patch: Vec<serde_json::Value>,
        expected: serde_json::Value,
        comment: Option<String>,
    },
    Failure {
        doc: serde_json::Value,
        patch: Vec<serde_json::Value>,
        error: String,
        comment: Option<String>,
    },
}

pub enum TestScope {
    Diff,
    Patch,
}

pub enum TestVariant {
    Tests,
    SpecTests,
}

pub fn load_json_patch_test_cases(
    scope: TestScope,
    variant: TestVariant,
) -> Vec<JsonPatchTestCase> {
    let json_data = match variant {
        TestVariant::Tests => super::JSON_PATCH_TESTS,
        TestVariant::SpecTests => super::JSON_PATCH_SPEC_TESTS,
    };
    let tests: Vec<serde_json::Value> =
        serde_json::from_str(json_data).expect("Failed to parse JSON patch tests");

    let mut test_cases = Vec::new();

    for test in tests {
        if should_be_disabled(&scope, &test) {
            continue;
        }
        let comment = test
            .get("comment")
            .and_then(|c| c.as_str())
            .map(|s| s.to_owned());

        let doc = test
            .get("doc")
            .cloned()
            .expect("Test case missing 'doc' field");
        let patch = test
            .get("patch")
            .and_then(|p| p.as_array())
            .cloned()
            .expect("Test case missing 'patch' field")
            .to_vec();

        if let Some(expected) = test.get("expected") {
            test_cases.push(JsonPatchTestCase::Valid {
                doc,
                patch,
                expected: expected.clone(),
                comment,
            });
        } else if let Some(error) = test.get("error").and_then(|e| e.as_str()) {
            test_cases.push(JsonPatchTestCase::Failure {
                doc,
                patch,
                error: error.to_owned(),
                comment,
            });
        } else {
            panic!(
                "Test case must have either 'expected' or 'error' field, {:?}",
                comment
            );
        }
    }

    test_cases
}

fn should_be_disabled(scope: &TestScope, test: &serde_json::Value) -> bool {
    if let Some(disabled) = test.get("disabled").and_then(|d| d.as_bool())
        && disabled
    {
        return true;
    }
    if let Some(disabled_scopes) = test.get("disabledScopes").and_then(|s| s.as_array()) {
        for scope_value in disabled_scopes {
            if let Some(scope_str) = scope_value.as_str() {
                match (scope_str, &scope) {
                    ("diff", TestScope::Diff) => return true,
                    ("patch", TestScope::Patch) => return true,
                    _ => {}
                }
            }
        }
    }
    false
}

pub fn load_json_patch_test_cases_for_diff() -> Vec<JsonPatchTestCase> {
    load_json_patch_test_cases(TestScope::Diff, TestVariant::Tests)
}
