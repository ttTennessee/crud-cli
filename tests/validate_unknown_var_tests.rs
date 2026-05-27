//! Unknown-variable validation tests (VAL-02).
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use crud_cli::core::config::SetupConfig;
use crud_cli::core::config::SetupSelections;
use crud_cli::core::config::{Backend, ComponentLibrary, Frontend, OverwritePolicy};
use crud_cli::core::error::Kind;
use crud_cli::core::validator::{run, ValidateParams};
use std::fs;
use std::sync::{Mutex, OnceLock};
use tempfile::TempDir;

static CWD_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

fn cwd_guard() -> std::sync::MutexGuard<'static, ()> {
    CWD_LOCK.get_or_init(|| Mutex::new(())).lock().unwrap()
}

fn seed_setup(root: &std::path::Path) -> SetupConfig {
    let crud = root.join(".crud");
    fs::create_dir_all(&crud).expect("mkdir .crud");
    let cfg = SetupConfig::from_selections(SetupSelections {
        backend: Backend::None,
        frontend: Frontend::None,
        component_library: ComponentLibrary::None,
        overwrite_policy: OverwritePolicy::Never,
    });
    fs::write(crud.join("setup.toml"), cfg.to_toml_pretty().unwrap()).expect("setup.toml");
    cfg
}

fn run_validate_in(root: &std::path::Path) -> Result<crud_cli::core::validator::ValidateReport, crud_cli::core::error::ErrorEnvelope> {
    let _lock = cwd_guard();
    let prev = std::env::current_dir().unwrap();
    std::env::set_current_dir(root).unwrap();
    let result = run(ValidateParams::default());
    std::env::set_current_dir(prev).unwrap();
    result
}

#[test]
fn unknown_first_segment_reported_with_didyoumean() {
    let dir = TempDir::new().unwrap();
    let root = dir.path();
    seed_setup(root);
    let templates = root.join(".crud/templates");
    fs::create_dir_all(&templates).unwrap();
    fs::write(templates.join("Bad.hbs"), "{{authr}}").unwrap();
    let mut cfg = SetupConfig::from_selections(SetupSelections {
        backend: Backend::None,
        frontend: Frontend::None,
        component_library: ComponentLibrary::None,
        overwrite_policy: OverwritePolicy::Never,
    });
    cfg.variables
        .0
        .insert("author".into(), toml::Value::String("x".into()));
    fs::write(root.join(".crud/setup.toml"), cfg.to_toml_pretty().unwrap()).unwrap();

    let err = run_validate_in(root).expect_err("should fail");
    assert_eq!(err.kind, Kind::TemplateError);
    let issues = err.details.get("issues").and_then(|v| v.as_array()).unwrap();
    let issue = issues
        .iter()
        .find(|i| i.get("kind").and_then(|k| k.as_str()) == Some("unknown_variable"))
        .expect("unknown_variable issue");
    assert_eq!(
        issue.get("variable").and_then(|v| v.as_str()),
        Some("authr")
    );
    let suggestion = issue
        .get("suggestion")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    assert!(
        suggestion.contains("author"),
        "suggestion should mention author: {suggestion}"
    );
}

#[test]
fn built_in_first_segment_allowed() {
    let dir = TempDir::new().unwrap();
    let root = dir.path();
    seed_setup(root);
    let templates = root.join(".crud/templates");
    fs::create_dir_all(&templates).unwrap();
    fs::write(
        templates.join("Ok.hbs"),
        "{{model}} {{package}} {{table}} {{package_path}}",
    )
    .unwrap();

    let report = run_validate_in(root).expect("should pass");
    assert_eq!(report.issue_count, 0);
}

#[test]
fn each_loop_locals_allowed() {
    let dir = TempDir::new().unwrap();
    let root = dir.path();
    seed_setup(root);
    let templates = root.join(".crud/templates");
    fs::create_dir_all(&templates).unwrap();
    fs::write(
        templates.join("Ok.hbs"),
        "{{#each fields}}{{name}}|{{@index}}{{/each}}",
    )
    .unwrap();

    let report = run_validate_in(root).expect("should pass");
    assert_eq!(report.issue_count, 0);
}

#[test]
fn nested_path_only_first_segment_checked() {
    let dir = TempDir::new().unwrap();
    let root = dir.path();
    seed_setup(root);
    let templates = root.join(".crud/templates");
    fs::create_dir_all(&templates).unwrap();
    fs::write(templates.join("Ok.hbs"), "{{author.email}}").unwrap();
    let mut cfg = SetupConfig::from_selections(SetupSelections {
        backend: Backend::None,
        frontend: Frontend::None,
        component_library: ComponentLibrary::None,
        overwrite_policy: OverwritePolicy::Never,
    });
    let mut table = toml::map::Map::new();
    table.insert("email".into(), toml::Value::String("a@b.c".into()));
    cfg.variables
        .0
        .insert("author".into(), toml::Value::Table(table));
    fs::write(root.join(".crud/setup.toml"), cfg.to_toml_pretty().unwrap()).unwrap();

    let report = run_validate_in(root).expect("should pass");
    assert_eq!(report.issue_count, 0);
}
