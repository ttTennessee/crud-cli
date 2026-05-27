//! Stdout/stderr policy and error envelope rendering (D-01, D-07, FOUND-09).

use crate::core::error::ErrorEnvelope;
use std::io::{self, Write};
use std::panic::PanicHookInfo;
use std::process;

use super::agent_mode::is_agent_active;

const ANSI_ESCAPE: &str = "\x1b[";

/// Panic hook handler → `InternalPanic` / exit 99 (D-04); wired from `main.rs` via `set_hook`.
pub fn panic_hook_handler(info: &PanicHookInfo<'_>) {
    let msg = panic_message(info);
    let location = info
        .location()
        .map(|l| format!("{}:{}:{}", l.file(), l.line(), l.column()));
    let thread = std::thread::current().name().map(str::to_owned);
    let envelope = ErrorEnvelope::internal_panic(msg, location.as_deref(), thread.as_deref());
    emit_failure(&envelope);
    process::exit(envelope.exit_code);
}

fn panic_message(info: &PanicHookInfo<'_>) -> String {
    if let Some(s) = info.payload().downcast_ref::<&str>() {
        return (*s).to_string();
    }
    if let Some(s) = info.payload().downcast_ref::<String>() {
        return s.clone();
    }
    "panic".to_string()
}

/// Renders failure to stderr only; stdout stays empty (D-01).
pub fn emit_failure(envelope: &ErrorEnvelope) {
    let agent = is_agent_active();
    let mut stderr = io::stderr().lock();
    if agent {
        let line = serde_json::to_string(envelope).unwrap_or_else(|_| {
            format!(
                "{{\"kind\":\"InternalPanic\",\"msg\":\"serialize error\",\"exit_code\":99,\"hint\":\"\",\"details\":{{}}}}"
            )
        });
        let _ = writeln!(stderr, "{line}");
    } else {
        let _ = writeln!(stderr, "error: {:?}", envelope.kind);
        let _ = writeln!(stderr, "msg: {}", envelope.msg);
        if !envelope.hint.is_empty() {
            let _ = writeln!(stderr, "hint: {}", envelope.hint);
        }
        let _ = writeln!(stderr, "exit_code: {}", envelope.exit_code);
    }
}

/// Success path: agent mode keeps stdout empty (FOUND-09).
pub fn emit_success(human_line: Option<&str>) {
    if is_agent_active() {
        return;
    }
    if let Some(line) = human_line {
        let mut stdout = io::stdout().lock();
        let _ = writeln!(stdout, "{line}");
    }
}

/// Whether output would contain ANSI escapes in agent mode (must be false).
#[must_use]
pub fn agent_output_suppresses_ansi() -> bool {
    is_agent_active()
}

/// Exposed for contract tests — builds panic envelope without unwinding.
#[must_use]
pub fn envelope_from_panic_payload(
    msg: &str,
    location: Option<&str>,
    thread: Option<&str>,
) -> ErrorEnvelope {
    ErrorEnvelope::internal_panic(msg, location, thread)
}

/// Formats envelope the same way as [`emit_failure`] would for agent mode.
#[must_use]
pub fn format_failure_agent_json(envelope: &ErrorEnvelope) -> String {
    serde_json::to_string(envelope).unwrap_or_else(|_| {
        r#"{"kind":"InternalPanic","msg":"serialize error","exit_code":99,"hint":"","details":{}}"#
            .to_string()
    })
}

#[must_use]
pub fn contains_ansi(s: &str) -> bool {
    s.contains(ANSI_ESCAPE)
}
