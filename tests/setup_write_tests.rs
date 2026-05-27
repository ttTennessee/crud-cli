//! Task 2: setup command write path and overwrite gates (CONF-08, FOUND-09).

use crud_cli::core::error::Kind;
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

fn full_setup_args(dir: &std::path::Path, extra: &[&str]) -> Command {
    let mut cmd = Command::new(exe());
    cmd.current_dir(dir).args([
        "setup",
        "--backend",
        "spring-boot",
        "--frontend",
        "vue",
        "--component-library",
        "element-plus",
    ]);
    cmd.args(extra);
    cmd
}

fn setup_toml_path(dir: &std::path::Path) -> std::path::PathBuf {
    dir.join(".crud").join("setup.toml")
}

#[test]
fn setup_existing_file_conflict() {
    let dir = TempDir::new().expect("tempdir");
    let path = setup_toml_path(dir.path());
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(&path, b"existing = true\n").unwrap();

    let output = full_setup_args(dir.path(), &["--overwrite-policy", "never"])
        .output()
        .expect("run setup");

    assert!(!output.status.success());
    assert_eq!(output.status.code(), Some(3));
    assert_eq!(fs::read(&path).unwrap(), b"existing = true\n");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("FileConflict") || stderr.contains("file exists") || stderr.contains("exists"));
}

#[test]
fn setup_force_only_requires_force_flag() {
    let dir = TempDir::new().expect("tempdir");
    let path = setup_toml_path(dir.path());
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(&path, b"old = 1\n").unwrap();

    let without_force = full_setup_args(dir.path(), &["--overwrite-policy", "force-only"])
        .output()
        .expect("run without force");
    assert_eq!(without_force.status.code(), Some(3));
    assert_eq!(fs::read(&path).unwrap(), b"old = 1\n");

    let with_force = full_setup_args(
        dir.path(),
        &["--overwrite-policy", "force-only", "--force"],
    )
    .output()
    .expect("run with force");
    assert!(with_force.status.success(), "stderr: {}", String::from_utf8_lossy(&with_force.stderr));
    let content = fs::read_to_string(&path).expect("updated file");
    assert!(content.contains("[project]"));
    assert!(!content.contains("old = 1"));
}

#[test]
fn setup_always_overwrites_existing() {
    let dir = TempDir::new().expect("tempdir");
    let path = setup_toml_path(dir.path());
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(&path, b"stale = true\n").unwrap();

    let output = full_setup_args(dir.path(), &["--overwrite-policy", "always"])
        .output()
        .expect("run always");
    assert!(output.status.success());
    let content = fs::read_to_string(&path).unwrap();
    assert!(content.contains("[project]"));
    assert!(!content.contains("stale"));
}

#[test]
fn setup_writes_config_successfully() {
    let dir = TempDir::new().expect("tempdir");
    let path = setup_toml_path(dir.path());
    assert!(!path.exists());

    let output = full_setup_args(dir.path(), &["--overwrite-policy", "never"])
        .output()
        .expect("run setup");
    assert!(output.status.success(), "stderr: {}", String::from_utf8_lossy(&output.stderr));
    assert!(path.is_file());
    let content = fs::read_to_string(&path).unwrap();
    assert!(content.contains("[project]"));
    assert!(content.contains("spring-boot"));
    assert!(content.contains("[paths]"));
    assert!(content.contains("[overwrite]"));
}

#[test]
fn agent_success_stdout_empty() {
    let _g = env_guard();
    let dir = TempDir::new().expect("tempdir");

    let output = Command::new(exe())
        .current_dir(dir.path())
        .env("CRUD_AGENT", "1")
        .args([
            "setup",
            "--backend",
            "none",
            "--frontend",
            "none",
            "--component-library",
            "none",
            "--overwrite-policy",
            "never",
        ])
        .output()
        .expect("run agent setup");

    assert!(output.status.success(), "stderr: {}", String::from_utf8_lossy(&output.stderr));
    assert!(
        output.stdout.is_empty(),
        "agent success stdout must be empty, got {:?}",
        String::from_utf8_lossy(&output.stdout)
    );
    assert!(setup_toml_path(dir.path()).is_file());

    let stderr = String::from_utf8_lossy(&output.stderr);
    if stderr.contains("\"kind\"") {
        assert_eq!(Kind::UserError.exit_code(), 1);
    }
}
