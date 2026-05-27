//! End-to-end tests for `validate` (Plan 02-03 Task 1).
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use crud_cli::cli::args::ValidateArgs;
use crud_cli::cli::commands::validate::run_validate;
use crud_cli::core::config::SetupConfig;
use crud_cli::core::config::SetupSelections;
use crud_cli::core::config::{Backend, ComponentLibrary, Frontend, OverwritePolicy};
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
        component_library: ComponentLibrary::None,
        overwrite_policy: OverwritePolicy::Never,
    });
    fs::write(crud.join("setup.toml"), cfg.to_toml_pretty().unwrap()).expect("setup.toml");
}

#[test]
fn validate_succeeds_on_well_formed_templates() {
    let dir = TempDir::new().unwrap();
    let root = dir.path();
    seed_setup(root);
    let templates = root.join(".crud/templates");
    fs::create_dir_all(&templates).unwrap();
    fs::write(
        templates.join("Entity.java.hbs"),
        "package {{package}}; class {{model_pascal}} {}",
    )
    .unwrap();

    let _lock = cwd_guard();
    let prev = std::env::current_dir().unwrap();
    std::env::set_current_dir(root).unwrap();
    let code = run_validate(ValidateArgs { type_: None });
    std::env::set_current_dir(prev).unwrap();

    assert_eq!(code, 0);
}

#[test]
fn validate_reports_no_templates_when_dir_missing() {
    let dir = TempDir::new().unwrap();
    let root = dir.path();
    seed_setup(root);

    let _lock = cwd_guard();
    let prev = std::env::current_dir().unwrap();
    std::env::set_current_dir(root).unwrap();
    let code = run_validate(ValidateArgs { type_: None });
    std::env::set_current_dir(prev).unwrap();

    assert_eq!(code, 1);
}
