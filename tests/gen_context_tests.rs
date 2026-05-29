//! `build_context` JSON shape tests (D-G04).
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use crud_cli::core::config::SetupConfig;
use crud_cli::core::config::SetupSelections;
use crud_cli::core::config::{Backend, Frontend};
use crud_cli::core::field_dsl::Field;
use crud_cli::core::gen_context::build_context_from_input;
use crud_cli::core::gen_input::GenInput;
use crud_cli::core::git_info::GitInfo;

#[test]
fn build_context_includes_model_and_field_case_keys() {
    let input = GenInput {
        name: "User".into(),
        table: "sys_user".into(),
        package: "com.acme.demo".into(),
        fields: vec![
            Field {
                name: "id".into(),
                ty: "Long".into(),
                is_pk: true,
                nullable: false,
            },
            Field {
                name: "first_name".into(),
                ty: "String".into(),
                is_pk: false,
                nullable: false,
            },
        ],
    };
    let setup = SetupConfig::from_selections(SetupSelections {
        backend: Backend::None,
        frontend: Frontend::None,
        template: None,
    });
    let ctx = build_context_from_input(
        &input,
        &setup,
        &GitInfo::default(),
        &crud_cli::core::gen_context::UserIdentity::default(),
    )
    .expect("context");
    let obj = ctx.as_object().expect("object");

    assert_eq!(obj.get("model").and_then(|v| v.as_str()), Some("User"));
    assert_eq!(
        obj.get("model_pascal").and_then(|v| v.as_str()),
        Some("User")
    );
    assert_eq!(
        obj.get("model_snake").and_then(|v| v.as_str()),
        Some("user")
    );
    assert_eq!(
        obj.get("package_path").and_then(|v| v.as_str()),
        Some("com/acme/demo")
    );

    let fields = obj
        .get("fields")
        .and_then(|v| v.as_array())
        .expect("fields");
    let first = fields[1].as_object().expect("field obj");
    assert_eq!(
        first.get("name_pascal").and_then(|v| v.as_str()),
        Some("FirstName")
    );
    assert_eq!(
        first.get("name_camel").and_then(|v| v.as_str()),
        Some("firstName")
    );
    assert_eq!(
        first.get("name_kebab").and_then(|v| v.as_str()),
        Some("first-name")
    );
    assert_eq!(
        first.get("name_snake").and_then(|v| v.as_str()),
        Some("first_name")
    );
}

#[test]
fn build_context_surfaces_field_comment_length_unique_default_from_json_spec() {
    use crud_cli::core::gen_context::{build_context, AsContextField};
    use crud_cli::core::gen_input::FieldSpec;

    let id = FieldSpec {
        name: "id".into(),
        ty: "Long".into(),
        is_pk: true,
        nullable: false,
        length: None,
        unique: false,
        default: None,
        comment: "主键".into(),
        extra: serde_json::Map::new(),
    };
    let email = FieldSpec {
        name: "email".into(),
        ty: "String".into(),
        is_pk: false,
        nullable: false,
        length: Some(128),
        unique: true,
        default: Some(serde_json::json!("n/a")),
        comment: "邮箱".into(),
        extra: serde_json::Map::new(),
    };
    let refs: Vec<&dyn AsContextField> = vec![&id, &email];
    let setup = SetupConfig::from_selections(SetupSelections {
        backend: Backend::None,
        frontend: Frontend::None,
        template: None,
    });
    let ctx = build_context(
        "User",
        "sys_user",
        "com.acme.demo",
        &refs,
        &setup,
        &GitInfo::default(),
        &crud_cli::core::gen_context::UserIdentity::default(),
    )
    .expect("context");
    let obj = ctx.as_object().expect("object");

    let fields = obj.get("fields").and_then(|v| v.as_array()).expect("fields");
    let f0 = fields[0].as_object().expect("f0");
    assert_eq!(f0.get("comment").and_then(|v| v.as_str()), Some("主键"));
    assert_eq!(f0.get("length"), Some(&serde_json::Value::Null));
    assert_eq!(f0.get("unique").and_then(|v| v.as_bool()), Some(false));

    let f1 = fields[1].as_object().expect("f1");
    assert_eq!(f1.get("comment").and_then(|v| v.as_str()), Some("邮箱"));
    assert_eq!(f1.get("length").and_then(|v| v.as_u64()), Some(128));
    assert_eq!(f1.get("unique").and_then(|v| v.as_bool()), Some(true));
    assert_eq!(f1.get("default").and_then(|v| v.as_str()), Some("n/a"));
}

#[test]
fn gen_report_serializes() {
    let report = crud_cli::core::gen_report::GenReport::default();
    let json = serde_json::to_string(&report).expect("serialize");
    assert!(json.contains("\"written\""));
}
