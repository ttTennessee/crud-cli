//! `_field_extra.toml` schema + MCP preview_entity_structure warning behavior.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use crud_cli::core::field_extra::SCHEMA_FILE_NAME;
use serde_json::Value;
use std::fs;
use tempfile::TempDir;

#[cfg(feature = "mcp")]
use crud_cli::core::config::{Backend, Frontend, SetupConfig, SetupSelections};

/// Minimal project with optional `_field_extra.toml`.
#[cfg(feature = "mcp")]
fn setup_project(extra_toml: Option<&str>) -> TempDir {
    let tmp = TempDir::new().expect("tmpdir");
    let crud = tmp.path().join(".crud");
    let templates = crud.join("templates");
    fs::create_dir_all(templates.join("java")).expect("templates dir");
    let cfg = SetupConfig::from_selections(SetupSelections {
        backend: Backend::None,
        frontend: Frontend::None,
        template: None,
    });
    fs::write(crud.join("setup.toml"), cfg.to_toml_pretty().unwrap()).expect("setup.toml");
    // describe_templates requires at least one .hbs file
    fs::write(templates.join("java/entity.java.hbs"), "{{name}}").expect("write hbs");
    if let Some(content) = extra_toml {
        fs::write(templates.join(SCHEMA_FILE_NAME), content).expect("write field_extra");
    }
    tmp
}

// ── load_schema unit behavior ────────────────────────────────────────────────

#[test]
fn no_schema_file_returns_empty_schema() {
    use crud_cli::core::field_extra::load_schema;
    let dir = TempDir::new().expect("tmp");
    let schema = load_schema(dir.path()).expect("load");
    assert!(schema.is_empty());
}

#[test]
fn valid_schema_loaded_correctly() {
    use crud_cli::core::field_extra::load_schema;
    let dir = TempDir::new().expect("tmp");
    fs::write(
        dir.path().join(SCHEMA_FILE_NAME),
        "[options]\ndescription = \"enum options\"\ntype = \"array\"\nrequired_for = [\"enum\"]\n",
    )
    .expect("write");
    let schema = load_schema(dir.path()).expect("load");
    assert!(!schema.is_empty());
    let def = schema.0.get("options").expect("options key");
    assert_eq!(def.description, "enum options");
    assert_eq!(def.required_for, vec!["enum"]);
}

#[test]
fn schema_rejects_empty_description() {
    use crud_cli::core::field_extra::load_schema;
    let dir = TempDir::new().expect("tmp");
    fs::write(
        dir.path().join(SCHEMA_FILE_NAME),
        "[options]\ndescription = \"\"\ntype = \"array\"\n",
    )
    .expect("write");
    let err = load_schema(dir.path()).expect_err("should fail");
    assert!(err.msg.contains("description"));
}

#[test]
fn schema_rejects_unknown_field_in_toml() {
    use crud_cli::core::field_extra::load_schema;
    let dir = TempDir::new().expect("tmp");
    fs::write(
        dir.path().join(SCHEMA_FILE_NAME),
        "[options]\ndescription = \"x\"\ntype = \"array\"\nunknown_field = true\n",
    )
    .expect("write");
    load_schema(dir.path()).expect_err("deny_unknown_fields should reject");
}

// ── describe_templates includes field_extra ──────────────────────────────────

#[cfg(feature = "mcp")]
#[test]
fn describe_templates_includes_empty_field_extra_when_no_schema() {
    use crud_cli::mcp::{describe_templates, load_project_context};
    let tmp = setup_project(None);
    let ctx = load_project_context(Some(tmp.path().to_path_buf())).expect("ctx");
    let out = describe_templates(&ctx).expect("describe");
    let fe = out.get("field_extra").expect("field_extra key present");
    assert!(fe.is_object());
    assert_eq!(fe.as_object().unwrap().len(), 0);
}

#[cfg(feature = "mcp")]
#[test]
fn describe_templates_includes_field_extra_schema() {
    use crud_cli::mcp::{describe_templates, load_project_context};
    let tmp = setup_project(Some(
        "[options]\ndescription = \"Enum options\"\ntype = \"array\"\nrequired_for = [\"enum\"]\n\
         [accept]\ndescription = \"MIME type filter\"\ntype = \"string\"\n",
    ));
    let ctx = load_project_context(Some(tmp.path().to_path_buf())).expect("ctx");
    let out = describe_templates(&ctx).expect("describe");

    let fe = out.get("field_extra").expect("field_extra key");
    assert!(fe.is_object());

    let options = fe.get("options").expect("options key");
    assert_eq!(options["description"], "Enum options");
    assert_eq!(options["type"], "array");
    assert_eq!(
        options["required_for"],
        Value::Array(vec!["enum".into()])
    );

    let accept = fe.get("accept").expect("accept key");
    assert_eq!(accept["type"], "string");
}

// ── validate_extra_keys logic ────────────────────────────────────────────────

fn make_spec(name: &str, ty: &str, extra: serde_json::Map<String, Value>) -> crud_cli::core::gen_input::FieldSpec {
    crud_cli::core::gen_input::FieldSpec {
        name: name.to_string(),
        ty: ty.to_string(),
        is_pk: false,
        nullable: false,
        required: false,
        length: None,
        unique: false,
        default: None,
        comment: String::new(),
        extra,
    }
}

#[test]
fn unknown_extra_key_detected() {
    use crud_cli::core::field_extra::{load_schema, validate_extra_keys};
    use serde_json::Map;
    let dir = TempDir::new().expect("tmp");
    fs::write(
        dir.path().join(SCHEMA_FILE_NAME),
        "[options]\ndescription = \"enum options\"\ntype = \"array\"\n",
    )
    .expect("write");
    let schema = load_schema(dir.path()).expect("load");
    let mut extra = Map::new();
    extra.insert("ghost_key".into(), Value::Bool(true));
    let spec = make_spec("status", "String", extra);
    let problems = validate_extra_keys(&schema, &[spec]);
    assert_eq!(problems.len(), 1);
    assert!(problems[0].contains("ghost_key"));
}

#[test]
fn missing_required_extra_key_detected() {
    use crud_cli::core::field_extra::{load_schema, validate_extra_keys};
    let dir = TempDir::new().expect("tmp");
    fs::write(
        dir.path().join(SCHEMA_FILE_NAME),
        "[options]\ndescription = \"enum options\"\ntype = \"array\"\nrequired_for = [\"enum\"]\n",
    )
    .expect("write");
    let schema = load_schema(dir.path()).expect("load");
    let spec = make_spec("status", "enum", serde_json::Map::new());
    let problems = validate_extra_keys(&schema, &[spec]);
    assert_eq!(problems.len(), 1);
    assert!(problems[0].contains("options"));
}

#[test]
fn no_schema_allows_any_extra_key() {
    use crud_cli::core::field_extra::{load_schema, validate_extra_keys};
    use serde_json::Map;
    let dir = TempDir::new().expect("tmp");
    let schema = load_schema(dir.path()).expect("load");
    let mut extra = Map::new();
    extra.insert("anything".into(), Value::String("x".into()));
    let spec = make_spec("foo", "String", extra);
    assert!(validate_extra_keys(&schema, &[spec]).is_empty());
}

#[test]
fn required_for_not_triggered_for_other_types() {
    use crud_cli::core::field_extra::{load_schema, validate_extra_keys};
    let dir = TempDir::new().expect("tmp");
    fs::write(
        dir.path().join(SCHEMA_FILE_NAME),
        "[options]\ndescription = \"enum options\"\ntype = \"array\"\nrequired_for = [\"enum\"]\n",
    )
    .expect("write");
    let schema = load_schema(dir.path()).expect("load");
    // type is String, not enum — requirement doesn't apply
    let spec = make_spec("name", "String", serde_json::Map::new());
    assert!(validate_extra_keys(&schema, &[spec]).is_empty());
}

// ── describe_templates field_extra is absent from old expected-absent keys ───

#[cfg(feature = "mcp")]
#[test]
fn describe_templates_no_raw_toml_keys() {
    use crud_cli::mcp::{describe_templates, load_project_context};
    let tmp = setup_project(None);
    let ctx = load_project_context(Some(tmp.path().to_path_buf())).expect("ctx");
    let out = describe_templates(&ctx).expect("describe");
    assert!(out.get("field_extra_toml").is_none());
}
