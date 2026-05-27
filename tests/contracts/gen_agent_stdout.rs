//! `gen` command agent-mode stdout contract (FOUND-09: empty success stdout; gap 02-04 option-a).
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

fn seed_gen_project(root: &std::path::Path) {
    let crud = root.join(".crud");
    fs::create_dir_all(crud.join("templates")).unwrap();
    fs::write(crud.join("templates/out.txt.hbs"), "{{model}}\n").unwrap();
    let cfg = SetupConfig::from_selections(SetupSelections {
        backend: Backend::None,
        frontend: Frontend::None,
        component_library: ComponentLibrary::None,
        overwrite_policy: OverwritePolicy::Never,
    });
    fs::write(crud.join("setup.toml"), cfg.to_toml_pretty().unwrap()).unwrap();
}

#[test]
fn gen_non_agent_success_stdout_line() {
    let _eg = env_guard();
    std::env::remove_var("CRUD_AGENT");

    let dir = tempfile::TempDir::new().unwrap();
    seed_gen_project(dir.path());

    let output = Command::new(exe())
        .current_dir(dir.path())
        .args([
            "gen",
            "User",
            "--fields",
            "id:Long",
            "--package",
            "com.x",
            "--table",
            "u",
        ])
        .output()
        .expect("run");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success());
    assert!(
        stdout.contains("生成"),
        "stdout should contain success line: {stdout}"
    );
    assert!(stdout.contains('1') || stdout.contains('一'));
}

#[test]
fn gen_agent_success_stdout_empty() {
    let _eg = env_guard();
    std::env::set_var("CRUD_AGENT", "1");

    let dir = tempfile::TempDir::new().unwrap();
    seed_gen_project(dir.path());

    let output = Command::new(exe())
        .current_dir(dir.path())
        .env("CRUD_AGENT", "1")
        .args([
            "gen",
            "User",
            "--fields",
            "id:Long",
            "--package",
            "com.x",
            "--table",
            "u",
        ])
        .output()
        .expect("run");

    // FOUND-09: agent success stdout is empty; file count lives in GenReport / future --json.
    assert!(output.stdout.is_empty(), "agent stdout must be empty");
    assert!(output.status.success());
    std::env::remove_var("CRUD_AGENT");
}

#[test]
fn gen_agent_failure_stdout_empty_json_stderr() {
    let _eg = env_guard();
    std::env::set_var("CRUD_AGENT", "1");

    let dir = tempfile::TempDir::new().unwrap();
    seed_gen_project(dir.path());

    let output = Command::new(exe())
        .current_dir(dir.path())
        .env("CRUD_AGENT", "1")
        .args([
            "gen",
            "User",
            "--fields",
            "",
            "--package",
            "com.x",
            "--table",
            "u",
        ])
        .output()
        .expect("run");

    assert!(output.stdout.is_empty());
    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    let line = stderr.lines().next().expect("stderr line");
    let v: Value = serde_json::from_str(line).expect("json envelope");
    assert_eq!(v.get("kind").and_then(|k| k.as_str()), Some("UserError"));
    assert_eq!(v.get("exit_code").and_then(|c| c.as_i64()), Some(1));
    std::env::remove_var("CRUD_AGENT");
}

#[test]
fn gen_non_agent_failure_human_stderr() {
    let _eg = env_guard();
    std::env::remove_var("CRUD_AGENT");

    let dir = tempfile::TempDir::new().unwrap();
    seed_gen_project(dir.path());

    let output = Command::new(exe())
        .current_dir(dir.path())
        .args([
            "gen",
            "User",
            "--fields",
            "",
            "--package",
            "com.x",
            "--table",
            "u",
        ])
        .output()
        .expect("run");

    assert!(output.stdout.is_empty());
    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("error:") || stderr.contains("msg:"));
}
