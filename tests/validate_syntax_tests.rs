//! Syntax validation tests (VAL-01).
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

#[test]
fn unclosed_if_block_reported() {
    let dir = TempDir::new().unwrap();
    let root = dir.path();
    seed_setup(root);
    let templates = root.join(".crud/templates");
    fs::create_dir_all(&templates).unwrap();
    fs::write(templates.join("Bad.hbs"), "{{#if x}}hi").unwrap();

    let _lock = cwd_guard();
    let prev = std::env::current_dir().unwrap();
    std::env::set_current_dir(root).unwrap();
    let result = run(ValidateParams::default());
    std::env::set_current_dir(prev).unwrap();

    let err = result.expect_err("validate should fail");
    assert_eq!(err.kind, Kind::TemplateError);
    assert_eq!(err.exit_code, 2);
    let issues = err
        .details
        .get("issues")
        .and_then(|v| v.as_array())
        .expect("issues array");
    assert!(!issues.is_empty());
    let first = &issues[0];
    assert_eq!(
        first.get("template_path").and_then(|v| v.as_str()),
        Some("Bad.hbs")
    );
    assert_eq!(
        first.get("kind").and_then(|v| v.as_str()),
        Some("syntax_error")
    );
    assert!(first.get("suggestion").and_then(|v| v.as_str()).is_some());
}
