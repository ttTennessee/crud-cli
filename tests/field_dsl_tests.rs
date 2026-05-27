//! `--fields` DSL parser tests (D-G07, D-G08).
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use crud_cli::core::error::Kind;
use crud_cli::core::field_dsl::{parse_fields, Field};

#[test]
fn parse_fields_happy_path() {
    let fields = parse_fields("id:Long,name:String").expect("ok");
    assert_eq!(
        fields,
        vec![
            Field {
                name: "id".into(),
                ty: "Long".into(),
                is_pk: false,
                nullable: false,
            },
            Field {
                name: "name".into(),
                ty: "String".into(),
                is_pk: false,
                nullable: false,
            },
        ]
    );
}

#[test]
fn parse_fields_pk_and_nullable() {
    let fields = parse_fields("*id:Long,email:String?").expect("ok");
    assert!(fields[0].is_pk);
    assert!(!fields[0].nullable);
    assert!(!fields[1].is_pk);
    assert!(fields[1].nullable);
}

fn assert_reason(src: &str, reason: &str) {
    let err = parse_fields(src).expect_err(reason);
    assert_eq!(err.kind, Kind::UserError);
    assert_eq!(err.exit_code, 1);
    assert_eq!(
        err.details.get("reason").and_then(|v| v.as_str()),
        Some(reason)
    );
}

#[test]
fn dsl_empty_type() {
    assert_reason("id:", "empty_type");
}

#[test]
fn dsl_empty_name() {
    assert_reason(":Long", "empty_name");
}

#[test]
fn dsl_invalid_identifier() {
    assert_reason("1st:Long", "invalid_identifier");
}

#[test]
fn dsl_too_many_segments() {
    assert_reason("name:String:extra", "too_many_segments");
}

#[test]
fn dsl_duplicate_field() {
    assert_reason("id:Long,id:String", "duplicate_field");
}

#[test]
fn dsl_no_fields() {
    assert_reason("", "no_fields");
}

#[test]
fn dsl_reserved_field_name() {
    assert_reason("model:Long", "reserved_field_name");
}
