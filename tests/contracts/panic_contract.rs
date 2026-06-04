//! Panic / kind / feature-gate contracts (01-01, 01-04).
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use crud_cli::core::error::Kind;
use std::process::Command;

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
fn handlebars_no_escape_contract() {
    let data = serde_json::json!({ "body": "<div>&\"</div>" });
    let out = crud_cli::core::template_engine::render_template("{{body}}", &data).expect("render");
    assert_eq!(out, "<div>&\"</div>");
    assert!(!out.contains("&lt;"));
    assert!(!out.contains("&gt;"));
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
