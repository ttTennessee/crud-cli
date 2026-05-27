//! Agent-mode output discipline contracts (D-01, FOUND-09/10).
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use crud_cli::core::error::{ErrorEnvelope, Kind};
use std::process::Command;
use std::sync::{Mutex, OnceLock};

static ENV_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

fn env_guard() -> std::sync::MutexGuard<'static, ()> {
    ENV_LOCK.get_or_init(|| Mutex::new(())).lock().unwrap()
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
fn agent_output_contract() {
    let _g = env_guard();
    std::env::set_var("CRUD_AGENT", "1");
    let envelope = ErrorEnvelope::internal_panic("x", None, None);
    let json = crud_cli::cli::output::format_failure_agent_json(&envelope);
    assert!(json.contains("\"kind\":\"InternalPanic\""));
    assert_eq!(envelope.kind, Kind::InternalPanic);
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
