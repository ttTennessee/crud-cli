//! `validate` command agent-mode stdout/stderr contract.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use crud_cli::core::config::SetupConfig;
use crud_cli::core::config::SetupSelections;
use crud_cli::core::config::{Backend, ComponentLibrary, Frontend, OverwritePolicy};
use serde_json::Value;
use std::fs;
use std::process::Command;
use std::sync::{Mutex, OnceLock};

static ENV_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

fn env_guard() -> std::sync::MutexGuard<'static, ()> {
    ENV_LOCK.get_or_init(|| Mutex::new(())).lock().unwrap()
}

fn exe() -> String {
    std::env::var("CARGO_BIN_EXE_crud_cli").unwrap_or_else(|_| {
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("target/debug/crud-cli")
            .to_string_lossy()
            .into_owned()
    })
}

fn seed_valid_project(root: &std::path::Path) {
    let crud = root.join(".crud");
    fs::create_dir_all(crud.join("templates")).unwrap();
    fs::write(
        crud.join("templates/Entity.java.hbs"),
        "package {{package}}; class {{model_pascal}} {}",
    )
    .unwrap();
    let cfg = SetupConfig::from_selections(SetupSelections {
        backend: Backend::None,
        frontend: Frontend::None,
        component_library: ComponentLibrary::None,
        overwrite_policy: OverwritePolicy::Never,
    });
    fs::write(crud.join("setup.toml"), cfg.to_toml_pretty().unwrap()).unwrap();
}

#[test]
fn success_silent_under_agent() {
    let _eg = env_guard();
    let dir = tempfile::TempDir::new().unwrap();
    seed_valid_project(dir.path());

    let output = Command::new(exe())
        .current_dir(dir.path())
        .env("CRUD_AGENT", "1")
        .args(["--agent", "validate"])
        .output()
        .expect("run");

    assert!(output.stdout.is_empty(), "stdout: {:?}", output.stdout);
    assert!(output.stderr.is_empty(), "stderr: {:?}", output.stderr);
    assert!(output.status.success());
}

#[test]
fn failure_stderr_envelope_under_agent() {
    let _eg = env_guard();
    let dir = tempfile::TempDir::new().unwrap();
    seed_valid_project(dir.path());
    fs::write(
        dir.path().join(".crud/templates/Bad.hbs"),
        "{{#if x}}unclosed",
    )
    .unwrap();

    let output = Command::new(exe())
        .current_dir(dir.path())
        .env("CRUD_AGENT", "1")
        .args(["--agent", "validate"])
        .output()
        .expect("run");

    assert!(output.stdout.is_empty());
    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&output.stderr);
    let line = stderr.lines().next().expect("stderr line");
    let v: Value = serde_json::from_str(line).expect("json envelope");
    assert_eq!(
        v.get("kind").and_then(|k| k.as_str()),
        Some("TemplateError")
    );
    assert_eq!(v.get("exit_code").and_then(|c| c.as_i64()), Some(2));
    assert!(v.get("details").and_then(|d| d.get("summary")).is_some());
    assert!(v.get("details").and_then(|d| d.get("issues")).is_some());
}
