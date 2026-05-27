//! Phase 01-01 process contract tests (D-01..D-07, FOUND-02/05/09/10).

use crud_cli::core::error::{ErrorEnvelope, Kind};
use std::process::Command;
use std::sync::{Mutex, OnceLock};

static ENV_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

fn env_guard() -> std::sync::MutexGuard<'static, ()> {
    ENV_LOCK.get_or_init(|| Mutex::new(())).lock().unwrap()
}

#[test]
fn kind_exit_code_contract() {
    assert_eq!(Kind::UserError.exit_code(), 1);
    assert_eq!(Kind::TemplateError.exit_code(), 2);
    assert_eq!(Kind::FileConflict.exit_code(), 3);
    assert_eq!(Kind::NetworkError.exit_code(), 4);
    assert_eq!(Kind::ConfigError.exit_code(), 5);
    assert_eq!(Kind::InternalPanic.exit_code(), 99);
}

#[test]
fn panic_contract() {
    let envelope = crud_cli::cli::output::envelope_from_panic_payload(
        "boom",
        Some("src/main.rs:1:1"),
        Some("main"),
    );
    assert_eq!(envelope.kind, Kind::InternalPanic);
    assert_eq!(envelope.exit_code, 99);
    assert_eq!(envelope.msg, "boom");
    assert_eq!(
        envelope.details.get("location").and_then(|v| v.as_str()),
        Some("src/main.rs:1:1")
    );
    let json = crud_cli::cli::output::format_failure_agent_json(&envelope);
    assert!(json.contains("\"kind\":\"InternalPanic\""));
    assert!(json.contains("\"exit_code\":99"));
}

#[test]
fn agent_mode_contract() {
    let _g = env_guard();
    std::env::remove_var("CRUD_AGENT");
    assert!(!crud_cli::cli::agent_mode::resolve_agent_mode(None));

    std::env::set_var("CRUD_AGENT", "1");
    assert!(crud_cli::cli::agent_mode::resolve_agent_mode(None));

    std::env::set_var("CRUD_AGENT", "1");
    assert!(!crud_cli::cli::agent_mode::resolve_agent_mode(Some(false)));

    std::env::set_var("CRUD_AGENT", "0");
    assert!(crud_cli::cli::agent_mode::resolve_agent_mode(Some(true)));

    std::env::remove_var("CRUD_AGENT");
}

#[test]
fn agent_output_no_ansi_contract() {
    let _g = env_guard();
    std::env::set_var("CRUD_AGENT", "1");
    assert!(crud_cli::cli::agent_mode::is_agent_from_env());
    let envelope = ErrorEnvelope::internal_panic("x", None, None);
    let json = crud_cli::cli::output::format_failure_agent_json(&envelope);
    assert!(!crud_cli::cli::output::contains_ansi(&json));
    std::env::remove_var("CRUD_AGENT");
}

#[test]
fn agent_success_stdout_empty() {
    let _g = env_guard();
    let exe = std::env::var("CARGO_BIN_EXE_crud_cli").unwrap_or_else(|_| {
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("target/debug/crud-cli")
            .to_string_lossy()
            .into_owned()
    });
    let output = Command::new(&exe)
        .env("CRUD_AGENT", "1")
        .output()
        .expect("run crud-cli binary");
    assert!(
        output.stdout.is_empty(),
        "agent success stdout must be empty, got {:?}",
        String::from_utf8_lossy(&output.stdout)
    );
    assert!(output.status.success());
    std::env::remove_var("CRUD_AGENT");
}

#[test]
fn cli_feature_gate_contract() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let status = Command::new("cargo")
        .args(["check", "--no-default-features", "--lib"])
        .current_dir(manifest_dir)
        .status()
        .expect("cargo check");
    assert!(status.success(), "core lib must build without cli deps");
}
