//! CLI-only wiring (`cli` Cargo feature).

#[cfg(feature = "cli")]
pub mod agent_mode;
#[cfg(feature = "cli")]
pub mod args;
#[cfg(feature = "cli")]
pub mod commands;
#[cfg(feature = "cli")]
pub mod output;
#[cfg(feature = "cli")]
pub mod setup_wizard;

pub use agent_mode::{init_agent_mode, is_agent_active};
pub use args::{
    exit_with_envelope, try_parse_cli, try_parse_cli_or_help, Cli, Commands, GenArgs, SetupArgs,
    ValidateArgs,
};
pub use commands::{run_gen, run_setup, run_validate};
pub use output::{emit_failure, emit_success, panic_hook_handler};
pub use setup_wizard::{
    run_project_wizard, run_user_wizard, selections_from_answers, user_selections_from_answers,
};
