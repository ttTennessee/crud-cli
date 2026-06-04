//! Front-matter validation tests (basePath / filename / YAML structural errors).
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use crud_cli::core::config::{Backend, Frontend, SetupConfig, SetupSelections};
use crud_cli::core::error::Kind;
use crud_cli::core::validator::{run, ValidateParams};
use std::fs;
use std::sync::{Mutex, OnceLock};
use tempfile::TempDir;

static CWD_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

fn cwd_guard() -> std::sync::MutexGuard<'static, ()> {
    CWD_LOCK.get_or_init(|| Mutex::new(())).lock().unwrap()
}

fn seed_setup(root: &std::path::Path) {
    let crud = root.join(".crud");
    fs::create_dir_all(&crud).unwrap();
    let cfg = SetupConfig::from_selections(SetupSelections {
        backend: Backend::None,
        frontend: Frontend::None,
        template: None,
    });
    fs::write(crud.join("setup.toml"), cfg.to_toml_pretty().unwrap()).unwrap();
}

fn run_validate_in(
    root: &std::path::Path,
) -> Result<crud_cli::core::validator::ValidateReport, crud_cli::core::error::ErrorEnvelope> {
    let _lock = cwd_guard();
    let prev = std::env::current_dir().unwrap();
    std::env::set_current_dir(root).unwrap();
    let result = run(ValidateParams::default());
    std::env::set_current_dir(prev).unwrap();
    result
}

fn find_issue<'a>(issues: &'a [serde_json::Value], kind: &str) -> Option<&'a serde_json::Value> {
    issues
        .iter()
        .find(|i| i.get("kind").and_then(|k| k.as_str()) == Some(kind))
}

#[test]
fn filename_with_slash_is_reported() {
    let dir = TempDir::new().unwrap();
    let root = dir.path();
    seed_setup(root);
    let templates = root.join(".crud/templates");
    fs::create_dir_all(&templates).unwrap();
    fs::write(
        templates.join("Bad.hbs"),
        "---\nfilename: \"sub/{{model_pascal}}.java\"\n---\nbody\n",
    )
    .unwrap();

    let err = run_validate_in(root).expect_err("should fail");
    assert_eq!(err.kind, Kind::TemplateError);
    let issues = err
        .details
        .get("issues")
        .and_then(|v| v.as_array())
        .unwrap();
    let issue = find_issue(issues, "invalid_filename").expect("invalid_filename issue");
    assert_eq!(
        issue.get("variable").and_then(|v| v.as_str()),
        Some("sub/ValidateFixture.java")
    );
}

#[test]
fn base_path_with_traversal_is_reported() {
    let dir = TempDir::new().unwrap();
    let root = dir.path();
    seed_setup(root);
    let templates = root.join(".crud/templates");
    fs::create_dir_all(&templates).unwrap();
    fs::write(
        templates.join("Bad.hbs"),
        "---\nbasePath: \"../escape/{{package_path}}\"\nfilename: \"X.java\"\n---\nbody\n",
    )
    .unwrap();

    let err = run_validate_in(root).expect_err("should fail");
    let issues = err
        .details
        .get("issues")
        .and_then(|v| v.as_array())
        .unwrap();
    find_issue(issues, "path_traversal").expect("path_traversal issue");
}

#[test]
fn base_path_absolute_is_reported() {
    let dir = TempDir::new().unwrap();
    let root = dir.path();
    seed_setup(root);
    let templates = root.join(".crud/templates");
    fs::create_dir_all(&templates).unwrap();
    let abs = if cfg!(windows) {
        "C:/abs/{{package_path}}"
    } else {
        "/abs/{{package_path}}"
    };
    fs::write(
        templates.join("Bad.hbs"),
        format!("---\nbasePath: \"{abs}\"\nfilename: \"X.java\"\n---\nbody\n"),
    )
    .unwrap();

    let err = run_validate_in(root).expect_err("should fail");
    let issues = err
        .details
        .get("issues")
        .and_then(|v| v.as_array())
        .unwrap();
    find_issue(issues, "path_traversal").expect("path_traversal for absolute");
}

#[test]
fn unclosed_front_matter_is_reported() {
    let dir = TempDir::new().unwrap();
    let root = dir.path();
    seed_setup(root);
    let templates = root.join(".crud/templates");
    fs::create_dir_all(&templates).unwrap();
    fs::write(
        templates.join("Bad.hbs"),
        "---\nfilename: \"X.java\"\nbody but no closing fence\n",
    )
    .unwrap();

    let err = run_validate_in(root).expect_err("should fail");
    let issues = err
        .details
        .get("issues")
        .and_then(|v| v.as_array())
        .unwrap();
    find_issue(issues, "front_matter_error").expect("front_matter_error issue");
}

#[test]
fn invalid_overwrite_policy_is_reported() {
    let dir = TempDir::new().unwrap();
    let root = dir.path();
    seed_setup(root);
    let templates = root.join(".crud/templates");
    fs::create_dir_all(&templates).unwrap();
    fs::write(
        templates.join("Bad.hbs"),
        "---\nfilename: \"X.java\"\noverwrite: \"sometimes\"\n---\nbody\n",
    )
    .unwrap();

    let err = run_validate_in(root).expect_err("should fail");
    let issues = err
        .details
        .get("issues")
        .and_then(|v| v.as_array())
        .unwrap();
    find_issue(issues, "front_matter_error").expect("front_matter_error issue");
}

#[test]
fn schema_declared_variable_is_allowed_in_template() {
    let dir = TempDir::new().unwrap();
    let root = dir.path();
    seed_setup(root);
    let templates = root.join(".crud/templates");
    fs::create_dir_all(&templates).unwrap();
    fs::write(
        templates.join("_variables.toml"),
        "[has_import]\ndescription = \"toggle import button\"\ntype = \"bool\"\ndefault = false\n",
    )
    .unwrap();
    fs::write(
        templates.join("Good.hbs"),
        "{{#if has_import}}IMPORT{{/if}}\n",
    )
    .unwrap();

    let report = run_validate_in(root).expect("should pass");
    assert_eq!(report.templates_checked, 1);
    assert_eq!(report.templates_with_issues, 0);
}

#[test]
fn undeclared_variable_in_template_is_unknown_var() {
    let dir = TempDir::new().unwrap();
    let root = dir.path();
    seed_setup(root);
    let templates = root.join(".crud/templates");
    fs::create_dir_all(&templates).unwrap();
    fs::write(templates.join("Bad.hbs"), "{{not_declared}}\n").unwrap();

    let err = run_validate_in(root).expect_err("should fail");
    let issues = err
        .details
        .get("issues")
        .and_then(|v| v.as_array())
        .unwrap();
    find_issue(issues, "unknown_variable").expect("unknown_variable issue");
}

#[test]
fn schema_with_invalid_type_is_rejected() {
    let dir = TempDir::new().unwrap();
    let root = dir.path();
    seed_setup(root);
    let templates = root.join(".crud/templates");
    fs::create_dir_all(&templates).unwrap();
    fs::write(
        templates.join("_variables.toml"),
        "[x]\ndescription = \"x\"\ntype = \"floaty\"\n",
    )
    .unwrap();
    fs::write(templates.join("Good.hbs"), "x\n").unwrap();

    run_validate_in(root).expect_err("should fail for bad schema type");
}

#[test]
fn well_formed_front_matter_passes() {
    let dir = TempDir::new().unwrap();
    let root = dir.path();
    seed_setup(root);
    let templates = root.join(".crud/templates");
    fs::create_dir_all(&templates).unwrap();
    fs::write(
        templates.join("Good.hbs"),
        "---\nbasePath: \"java/{{package_path}}/controller\"\nfilename: \"{{model_pascal}}.java\"\n---\nbody {{model}}\n",
    )
    .unwrap();

    let report = run_validate_in(root).expect("should pass");
    assert_eq!(report.templates_checked, 1);
    assert_eq!(report.templates_with_issues, 0);
}
