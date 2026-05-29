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
    ensure_language_preference, prompt_language, run_project_wizard, run_user_wizard,
    selections_from_answers, user_selections_from_answers,
};

use crate::core::global_config::{resolve_preferred_lang, GlobalConfig};
use crate::core::i18n::{self, Lang};
use crate::core::paths::global_config_toml;

/// Resolves and applies the active UI locale for the process.
///
/// Agent mode always pins English (deterministic JSON). Otherwise the locale
/// follows `CRUD_LANG` → `~/.crud/config.toml`, defaulting to English when no
/// preference exists yet (the `setup` command prompts to record one).
pub fn init_locale() {
    if is_agent_active() {
        i18n::set(Lang::En);
        return;
    }
    let cfg = match global_config_toml() {
        Ok(path) => GlobalConfig::load_or_default(&path),
        Err(_) => GlobalConfig::default(),
    };
    if let Some(lang) = resolve_preferred_lang(&cfg) {
        i18n::set(lang);
    }
}
