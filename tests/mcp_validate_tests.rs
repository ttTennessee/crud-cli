//! MCP entity validation reuses core schema checks (no code generation).
#![allow(clippy::unwrap_used, clippy::expect_used)]

use crud_cli::mcp::{load_project_context, validate_entity_json};
use std::collections::BTreeMap;
use std::path::Path;

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
