//! Render-phase validation tests (VAL-03).
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use crud_cli::core::config::SetupConfig;
use crud_cli::core::config::SetupSelections;
use crud_cli::core::config::{Backend, Frontend, OverwritePolicy};
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
    fs::create_dir_all(&crud).expect("mkdir .crud");
    let cfg = SetupConfig::from_selections(SetupSelections {
        backend: Backend::None,
        frontend: Frontend::None,
        template: None,
    });
    fs::write(crud.join("setup.toml"), cfg.to_toml_pretty().unwrap()).expect("setup.toml");
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
fn missing_helper_reported() {
    let dir = TempDir::new().unwrap();
    let root = dir.path();
    seed_setup(root);
    let templates = root.join(".crud/templates");
    fs::create_dir_all(&templates).unwrap();
    fs::write(templates.join("Bad.hbs"), "{{date_helper model}}").unwrap();

    let err = run_validate_in(root).expect_err("should fail");
    assert_eq!(err.kind, Kind::TemplateError);
    let issues = err.details.get("issues").and_then(|v| v.as_array()).unwrap();
    let issue = issues
        .iter()
        .find(|i| i.get("kind").and_then(|k| k.as_str()) == Some("missing_helper"))
        .expect("missing_helper issue");
    assert_eq!(
        issue.get("variable").and_then(|v| v.as_str()),
        Some("date_helper")
    );
}

#[test]
fn render_passes_on_field_each_with_fixture() {
    let dir = TempDir::new().unwrap();
    let root = dir.path();
    seed_setup(root);
    let templates = root.join(".crud/templates");
    fs::create_dir_all(&templates).unwrap();
    fs::write(
        templates.join("Ok.hbs"),
        "{{#each fields}}{{name}}{{/each}}",
    )
    .unwrap();

    let report = run_validate_in(root).expect("should pass");
    assert_eq!(report.issue_count, 0);
}

#[test]
fn aggregate_multiple_templates() {
    let dir = TempDir::new().unwrap();
    let root = dir.path();
    seed_setup(root);
    let templates = root.join(".crud/templates");
    fs::create_dir_all(&templates).unwrap();
    fs::write(
        templates.join("Good.hbs"),
        "package {{package}}; class {{model_pascal}} {}",
    )
    .unwrap();
    fs::write(templates.join("BadSyntax.hbs"), "{{#if}}").unwrap();
    fs::write(templates.join("BadVar.hbs"), "{{nonexistent}}").unwrap();

    let err = run_validate_in(root).expect_err("should fail");
    assert_eq!(err.exit_code, 2);
    let summary = err.details.get("summary").expect("summary");
    assert_eq!(summary.get("templates_checked").and_then(|v| v.as_u64()), Some(3));
    assert_eq!(
        summary.get("templates_with_issues").and_then(|v| v.as_u64()),
        Some(2)
    );
    assert_eq!(summary.get("issue_count").and_then(|v| v.as_u64()), Some(2));
    let issues = err.details.get("issues").and_then(|v| v.as_array()).unwrap();
    assert_eq!(issues.len(), 2);
    let kinds: Vec<_> = issues
        .iter()
        .filter_map(|i| i.get("kind").and_then(|k| k.as_str()))
        .collect();
    assert!(kinds.contains(&"syntax_error"));
    assert!(kinds.contains(&"unknown_variable"));
}
