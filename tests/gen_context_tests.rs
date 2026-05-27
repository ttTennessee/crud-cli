//! `build_context` JSON shape tests (D-G04).
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use crud_cli::core::config::SetupConfig;
use crud_cli::core::config::SetupSelections;
use crud_cli::core::config::{Backend, ComponentLibrary, Frontend, OverwritePolicy};
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
        component_library: ComponentLibrary::None,
        overwrite_policy: OverwritePolicy::Never,
    });
    let ctx = build_context_from_input(&input, &setup, &GitInfo::default()).expect("context");
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

    let fields = obj.get("fields").and_then(|v| v.as_array()).expect("fields");
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
fn gen_report_serializes() {
    let report = crud_cli::core::gen_report::GenReport::default();
    let json = serde_json::to_string(&report).expect("serialize");
    assert!(json.contains("\"written\""));
}
