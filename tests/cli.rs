//! End-to-end tests for the `spatch` binary.

use std::{
    path::PathBuf,
    process::{Command, Output},
};

/// Writes `contents` to a uniquely named temp file and returns its path.
fn write_json(name: &str, contents: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!("spatch-cli-{}-{name}", std::process::id()));
    std::fs::write(&path, contents).unwrap();
    path
}

/// Runs `spatch diff <left> <right> <extra...>`.
fn diff(left: &PathBuf, right: &PathBuf, extra: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_spatch"))
        .arg("diff")
        .arg(left)
        .arg(right)
        .args(extra)
        .output()
        .unwrap()
}

fn stdout(out: &Output) -> String {
    String::from_utf8(out.stdout.clone()).unwrap()
}

fn stderr(out: &Output) -> String {
    String::from_utf8(out.stderr.clone()).unwrap()
}

const LEFT: &str = r#"[{"a":1,"b":2,"c":3}]"#;
const RIGHT: &str = r#"[{"a":9,"b":8,"c":7}]"#;

#[test]
fn granular_should_keep_nested_operations_where_default_replaces_the_object() {
    let left = write_json("g-l.json", LEFT);
    let right = write_json("g-r.json", RIGHT);

    let default = stdout(&diff(&left, &right, &[]));
    let compact = stdout(&diff(&left, &right, &["--granularity", "compact"]));
    let granular = stdout(&diff(&left, &right, &["--granularity", "granular"]));

    assert_eq!(default, compact);
    assert!(default.contains(r#""path": "/0""#), "{default}");
    assert!(granular.contains(r#""path": "/0/a""#), "{granular}");
}

#[test]
fn invalid_granularity_should_be_rejected_with_the_accepted_values() {
    let left = write_json("i-l.json", LEFT);
    let right = write_json("i-r.json", RIGHT);

    let out = diff(&left, &right, &["--granularity", "bogus"]);

    let stderr = stderr(&out);
    assert!(!out.status.success());
    assert!(
        stderr.contains("compact") && stderr.contains("granular"),
        "{stderr}"
    );
}

#[test]
fn failing_diff_should_print_every_error_one_per_line_and_exit_1() {
    let schema = write_json(
        "e-s.json",
        r#"{"type":"object","properties":{"n":{"type":"array","x-spatch-indexKey":"id","items":{"type":"object"}}}}"#,
    );
    let left = write_json("e-l.json", r#"{"n":[{"x":1},{"x":2}]}"#);
    let right = write_json("e-r.json", r#"{"n":[{"x":1}]}"#);

    let out = diff(&left, &right, &["--schema", schema.to_str().unwrap()]);

    let stderr = stderr(&out);
    assert_eq!(out.status.code(), Some(1));
    assert!(
        stderr.contains("left: Item /n/0 is missing index key 'id'"),
        "{stderr}"
    );
    assert!(
        stderr.contains("left: Item /n/1 is missing index key 'id'"),
        "{stderr}"
    );
    assert!(
        stderr.contains("right: Item /n/0 is missing index key 'id'"),
        "{stderr}"
    );
    assert!(!stderr.contains("DiffErrorSummary"), "{stderr}");
    assert!(out.stdout.is_empty());
}
