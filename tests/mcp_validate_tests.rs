//! MCP entity validation reuses core schema checks (no code generation).
#![allow(clippy::unwrap_used, clippy::expect_used)]

use crud_cli::core::config::{Backend, Frontend, SetupConfig, SetupSelections};
use crud_cli::mcp::{describe_templates, load_project_context, validate_entity_json};
use serde_json::Value;
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use tempfile::TempDir;

#[test]
fn validate_entity_json_accepts_minimal_entity() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let ctx = load_project_context(Some(root.to_path_buf())).expect("project ctx");
    let json = r#"{
        "name": "User",
        "table": "sys_user",
        "package": "com.acme.demo",
        "fields": [
            { "name": "id", "type": "Long", "is_pk": true, "comment": "主键" }
        ],
        "variables": {
            "module_name": "system",
            "permission_prefix": "system:user"
        }
    }"#;
    validate_entity_json(&ctx, json, &BTreeMap::new()).expect("valid entity");
}

#[test]
fn validate_entity_json_rejects_unknown_field_key() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let ctx = load_project_context(Some(root.to_path_buf())).expect("project ctx");
    let json = r#"{
        "name": "User",
        "table": "sys_user",
        "package": "com.acme.demo",
        "fields": [
            { "name": "id", "type": "Long", "is_pk": true, "typo_key": true }
        ]
    }"#;
    let err = validate_entity_json(&ctx, json, &BTreeMap::new()).expect_err("unknown field");
    assert_eq!(err.kind, crud_cli::core::error::Kind::UserError);
}

#[test]
fn describe_templates_returns_parsed_schema_json() {
    let tmp = TempDir::new().expect("tmpdir");
    let crud = tmp.path().join(".crud");
    let templates = crud.join("templates");
    fs::create_dir_all(&templates).expect("templates dir");
    let cfg = SetupConfig::from_selections(SetupSelections {
        backend: Backend::None,
        frontend: Frontend::None,
        template: None,
    });
    fs::write(crud.join("setup.toml"), cfg.to_toml_pretty().unwrap()).expect("setup.toml");
    fs::create_dir_all(templates.join("java")).expect("mkdir java");
    std::fs::write(
        templates.join("_variables.toml"),
        r#"
[module_name]
description = "模块名"
type = "string"
required = true
"#,
    )
    .expect("write variables");
    std::fs::write(
        templates.join("_field_types.toml"),
        r#"
[Long]
description = "64-bit integer"
aliases = ["long"]
"#,
    )
    .expect("write field types");
    std::fs::write(templates.join("java/entity.java.hbs"), "{{name}}").expect("write hbs");

    let ctx = load_project_context(Some(tmp.path().to_path_buf())).expect("ctx");
    let out = describe_templates(&ctx).expect("describe");

    let vars = out.get("variables").expect("variables key");
    assert!(vars.is_object());
    assert_eq!(vars["module_name"]["type"], "string");
    assert_eq!(vars["module_name"]["required"], true);

    let types = out.get("field_types").expect("field_types key");
    assert!(types.is_object());
    assert_eq!(types["Long"]["description"], "64-bit integer");
    assert_eq!(types["Long"]["aliases"], Value::Array(vec!["long".into()]));

    assert!(out.get("variables_toml").is_none());
    assert!(out.get("field_types_toml").is_none());
}
