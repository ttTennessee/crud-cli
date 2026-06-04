//! Setup command write path: project + user scopes, confirm/force semantics.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::fs;
use std::process::Command;
use std::sync::{Mutex, OnceLock};
use tempfile::TempDir;

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

fn project_setup_args(dir: &std::path::Path, extra: &[&str]) -> Command {
    let mut cmd = Command::new(exe());
    cmd.current_dir(dir).args([
        "setup",
        "--project",
        "--backend",
        "java",
        "--frontend",
        "vue",
    ]);
    cmd.args(extra);
    cmd
}

fn user_setup_args(dir: &std::path::Path, extra: &[&str]) -> Command {
    let mut cmd = Command::new(exe());
    cmd.current_dir(dir).args([
        "setup",
        "--user-name",
        "Alice",
        "--user-email",
        "a@example.com",
        "--overwrite-policy",
        "never",
        "--enabled-types",
        "all",
    ]);
    cmd.args(extra);
    cmd
}

fn setup_toml_path(dir: &std::path::Path) -> std::path::PathBuf {
    dir.join(".crud").join("setup.toml")
}

fn user_toml_path(dir: &std::path::Path) -> std::path::PathBuf {
    dir.join(".crud").join("setup.user.toml")
}

fn gitignore_path(dir: &std::path::Path) -> std::path::PathBuf {
    dir.join(".crud").join(".gitignore")
}

#[test]
fn project_setup_existing_blocks_without_force_in_non_tty() {
    let dir = TempDir::new().expect("tempdir");
    let path = setup_toml_path(dir.path());
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(&path, b"existing = true\n").unwrap();

    let output = project_setup_args(dir.path(), &[])
        .output()
        .expect("run setup");
    assert!(!output.status.success());
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(fs::read(&path).unwrap(), b"existing = true\n");
}

#[test]
fn project_setup_force_overwrites_existing() {
    let dir = TempDir::new().expect("tempdir");
    let path = setup_toml_path(dir.path());
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(&path, b"stale = true\n").unwrap();

    let output = project_setup_args(dir.path(), &["--force"])
        .output()
        .expect("run with force");
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let content = fs::read_to_string(&path).unwrap();
    assert!(content.contains("[project]"));
    assert!(!content.contains("stale"));
    assert!(!content.contains("[overwrite]"));
}

#[test]
fn project_setup_writes_config_successfully() {
    let dir = TempDir::new().expect("tempdir");
    let path = setup_toml_path(dir.path());
    assert!(!path.exists());

    let output = project_setup_args(dir.path(), &[])
        .output()
        .expect("run setup");
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let content = fs::read_to_string(&path).unwrap();
    assert!(content.contains("[project]"));
    assert!(content.contains("backend = \"java\""));
    assert!(content.contains("[paths.lang]"));
}

#[test]
fn user_setup_writes_and_seeds_gitignore() {
    let dir = TempDir::new().expect("tempdir");
    let output = user_setup_args(dir.path(), &[])
        .output()
        .expect("run user setup");
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let user = fs::read_to_string(user_toml_path(dir.path())).unwrap();
    assert!(user.contains("[user]"));
    assert!(user.contains("[overwrite]"));
    let gi = fs::read_to_string(gitignore_path(dir.path())).unwrap();
    assert!(gi.lines().any(|l| l.trim() == "setup.user.toml"));
}

#[test]
fn user_setup_gitignore_is_idempotent() {
    let dir = TempDir::new().expect("tempdir");
    let _ = user_setup_args(dir.path(), &[])
        .output()
        .expect("first run");
    let _ = user_setup_args(dir.path(), &["--force"])
        .output()
        .expect("second run with force");
    let gi = fs::read_to_string(gitignore_path(dir.path())).unwrap();
    let count = gi.lines().filter(|l| l.trim() == "setup.user.toml").count();
    assert_eq!(count, 1, "duplicate gitignore line: {gi}");
}

#[test]
fn user_setup_existing_blocks_without_force_in_non_tty() {
    let dir = TempDir::new().expect("tempdir");
    let _ = user_setup_args(dir.path(), &[])
        .output()
        .expect("first run");
    let path = user_toml_path(dir.path());
    let before = fs::read(&path).unwrap();

    let output = user_setup_args(dir.path(), &[])
        .output()
        .expect("second run");
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(fs::read(&path).unwrap(), before);
}

#[test]
fn agent_success_stdout_empty_via_project_setup() {
    let _g = env_guard();
    let dir = TempDir::new().expect("tempdir");

    let output = Command::new(exe())
        .current_dir(dir.path())
        .env("CRUD_AGENT", "1")
        .args([
            "setup",
            "--project",
            "--backend",
            "none",
            "--frontend",
            "none",
        ])
        .output()
        .expect("run agent setup");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.stdout.is_empty(),
        "agent success stdout must be empty, got {:?}",
        String::from_utf8_lossy(&output.stdout)
    );
    assert!(setup_toml_path(dir.path()).is_file());
}
