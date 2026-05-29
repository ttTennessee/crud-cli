//! `--agent` / `CRUD_AGENT` detection (D-05).

use std::sync::OnceLock;

static CLI_AGENT_FLAG: OnceLock<Option<bool>> = OnceLock::new();

/// Returns true when `CRUD_AGENT=1` (exact match).
#[must_use]
pub fn is_agent_from_env() -> bool {
    matches!(std::env::var("CRUD_AGENT"), Ok(v) if v == "1")
}

/// Records CLI `--agent` flag; `Some(true|false)` overrides env when set.
pub fn init_agent_mode(cli_flag: Option<bool>) {
    let _ = CLI_AGENT_FLAG.set(cli_flag);
}

/// Active agent mode: CLI flag wins over `CRUD_AGENT` .
#[must_use]
pub fn is_agent_active() -> bool {
    match CLI_AGENT_FLAG.get().and_then(|o| *o) {
        Some(flag) => flag,
        None => is_agent_from_env(),
    }
}

/// Resolves agent mode for tests and pre-parse bootstrap.
#[must_use]
pub fn resolve_agent_mode(cli_flag: Option<bool>) -> bool {
    match cli_flag {
        Some(v) => v,
        None => is_agent_from_env(),
    }
}
