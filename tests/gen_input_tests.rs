//! JSON `--file` loader closed-set errors (D-G18).
#![allow(clippy::unwrap_used, clippy::expect_used)]

use crud_cli::core::error::Kind;
use crud_cli::core::gen_input::{load_gen_input_from_json, GenCliOverrides};
use std::io::Write;
use tempfile::NamedTempFile;

fn assert_reason(err: &crud_cli::core::error::ErrorEnvelope, reason: &str) {
    assert_eq!(err.kind, Kind::UserError);
    assert_eq!(
        err.details.get("reason").and_then(|v| v.as_str()),
        Some(reason)
    );
}

#[test]
fn json_missing_field() {
    let mut f = NamedTempFile::new().expect("temp");
    writeln!(
        f,
        r#"{{
  "name": "User",
  "table": "u",
  "package": "com.x",
  "fields": [{{ "type": "Long" }}]
}}"#
    )
    .unwrap();
    let err = load_gen_input_from_json(f.path(), GenCliOverrides::default()).expect_err("err");
    assert_reason(&err, "missing_field");
}

#[test]
fn json_unknown_field_did_you_mean() {
    let mut f = NamedTempFile::new().expect("temp");
    writeln!(
        f,
        r#"{{
  "name": "User",
  "table": "u",
  "package": "com.x",
  "fields": [{{ "name": "id", "type": "Long", "requried": true }}]
}}"#
    )
    .unwrap();
    let err = load_gen_input_from_json(f.path(), GenCliOverrides::default()).expect_err("err");
    assert_reason(&err, "unknown_field");
    assert!(
        err.hint.contains("required"),
        "hint should suggest required, got: {}",
        err.hint
    );
}

#[test]
fn json_type_mismatch() {
    let mut f = NamedTempFile::new().expect("temp");
    writeln!(
        f,
        r#"{{
  "name": "User",
  "table": "u",
  "package": "com.x",
  "fields": [{{ "name": "id", "type": "Long", "is_pk": "yes" }}]
}}"#
    )
    .unwrap();
    let err = load_gen_input_from_json(f.path(), GenCliOverrides::default()).expect_err("err");
    assert_reason(&err, "type_mismatch");
}

#[test]
fn json_file_not_found() {
    let err = load_gen_input_from_json(
        std::path::Path::new("/nonexistent/crud-user.json"),
        GenCliOverrides::default(),
    )
    .expect_err("err");
    assert_reason(&err, "file_not_found");
}

#[test]
fn json_invalid_json_syntax() {
    let mut f = NamedTempFile::new().expect("temp");
    writeln!(f, "{{ not json").unwrap();
    let err = load_gen_input_from_json(f.path(), GenCliOverrides::default()).expect_err("err");
    assert_reason(&err, "invalid_json");
}

#[test]
fn json_cli_override_wins() {
    let mut f = NamedTempFile::new().expect("temp");
    writeln!(
        f,
        r#"{{
  "name": "FromJson",
  "table": "t_json",
  "package": "com.json",
  "fields": [{{ "name": "id", "type": "Long" }}]
}}"#
    )
    .unwrap();
    let input = load_gen_input_from_json(
        f.path(),
        GenCliOverrides {
            name: Some("FromCli".into()),
            package: Some("com.cli".into()),
            table: Some("t_cli".into()),
            table_comment: None,
        },
    )
    .expect("ok");
    assert_eq!(input.name, "FromCli");
    assert_eq!(input.package, "com.cli");
    assert_eq!(input.table, "t_cli");
}

#[test]
fn json_load_success() {
    let mut f = NamedTempFile::new().expect("temp");
    writeln!(
        f,
        r#"{{
  "name": "User",
  "table": "sys_user",
  "package": "com.acme",
  "fields": [{{ "name": "id", "type": "Long", "is_pk": true }}]
}}"#
    )
    .unwrap();
    let input = load_gen_input_from_json(f.path(), GenCliOverrides::default()).expect("ok");
    assert_eq!(input.name, "User");
    assert_eq!(input.fields.len(), 1);
    assert!(input.fields[0].is_pk);
}
