//! CLI-only wiring (`cli` Cargo feature).

#[cfg(feature = "cli")]
pub mod agent_mode;
#[cfg(feature = "cli")]
pub mod output;

pub use agent_mode::{init_agent_mode, is_agent_active};
pub use output::{emit_failure, emit_success, panic_hook_handler};
