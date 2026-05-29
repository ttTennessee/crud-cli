//! `crud-cli setup` execution pipeline (D-10, CONF-08, FOUND-09).
//!
//! Defaults to writing the per-developer user file `.crud/setup.user.toml`.
//! Pass `--project` to write the shared `.crud/setup.toml`.

use std::env;
use std::io::IsTerminal;
use std::path::{Path, PathBuf};

use inquire::Confirm;

use crate::core::error::{ErrorEnvelope, Kind};
use crate::core::fs_writer::{commit, plan, OverwriteContext, WriteTarget};
use crate::core::paths::{
    ensure_gitignore_entry, project_crud_gitignore, project_setup_toml, project_setup_user_toml,
};

use crate::cli::agent_mode::is_agent_active;
use crate::cli::args::{exit_with_envelope, SetupArgs};
use crate::cli::output::emit_success;
use crate::cli::setup_wizard::{ensure_language_preference, run_project_wizard, run_user_wizard};
use crate::core::i18n::{self, keys};

const SETUP_USER_FILE: &str = "setup.user.toml";

/// Routes setup to either the project or the user file.
pub fn run_setup(setup: SetupArgs) -> i32 {
    let project_root = match env::current_dir() {
        Ok(p) => p,
        Err(e) => {
            let env = ErrorEnvelope {
                kind: Kind::ConfigError,
                msg: format!("resolve project root: {e}"),
                exit_code: Kind::ConfigError.exit_code(),
                hint: String::new(),
                details: serde_json::Map::new(),
            };
            return exit_with_envelope(&env);
        }
    };

    if setup.writes_project() {
        run_project_setup(&project_root, setup)
    } else {
        run_user_setup(&project_root, setup)
    }
}

fn run_project_setup(project_root: &Path, args: SetupArgs) -> i32 {
    let target = project_setup_toml(project_root);
    match check_overwrite_confirm(&target, args.force) {
        Decision::Skip => return 0,
        Decision::Block(env) => return exit_with_envelope(&env),
        Decision::Proceed => {}
    }
    let config_result = if args.is_project_non_interactive() {
        args.to_setup_config()
    } else {
        // First-run language detection runs only before the interactive wizard.
        ensure_language_preference();
        run_project_wizard()
    };
    match config_result {
        Ok(cfg) => match cfg.to_toml_pretty().and_then(|s| write_atomic(&target, s.into_bytes())) {
            Ok(()) => {
                let line = i18n::tf(
                    keys::SETUP_CREATED,
                    &[("path", &target.display().to_string())],
                );
                emit_success(Some(&line));
                0
            }
            Err(env) => exit_with_envelope(&env),
        },
        Err(env) => exit_with_envelope(&env),
    }
}

fn run_user_setup(project_root: &Path, args: SetupArgs) -> i32 {
    let target = project_setup_user_toml(project_root);
    match check_overwrite_confirm(&target, args.force) {
        Decision::Skip => return 0,
        Decision::Block(env) => return exit_with_envelope(&env),
        Decision::Proceed => {}
    }
    let config_result = if args.is_user_non_interactive() {
        args.to_user_config()
    } else {
        // First-run language detection runs only before the interactive wizard.
        ensure_language_preference();
        run_user_wizard()
    };
    let cfg = match config_result {
        Ok(c) => c,
        Err(env) => return exit_with_envelope(&env),
    };

    let toml_bytes = match cfg.to_toml_pretty() {
        Ok(s) => s.into_bytes(),
        Err(env) => return exit_with_envelope(&env),
    };

    if let Err(env) = write_atomic(&target, toml_bytes) {
        return exit_with_envelope(&env);
    }

    let gitignore_path = project_crud_gitignore(project_root);
    if let Err(env) = ensure_gitignore_entry(&gitignore_path, SETUP_USER_FILE) {
        return exit_with_envelope(&env);
    }

    let mut human = i18n::tf(
        keys::SETUP_CREATED,
        &[("path", &target.display().to_string())],
    );
    if !project_setup_toml(project_root).exists() {
        human.push_str(i18n::t(keys::SETUP_CREATED_NO_PROJECT_HINT));
    }
    emit_success(Some(&human));
    0
}

enum Decision {
    Proceed,
    Skip,
    Block(ErrorEnvelope),
}

fn check_overwrite_confirm(target: &Path, force: bool) -> Decision {
    if !target.exists() {
        return Decision::Proceed;
    }
    if force {
        return Decision::Proceed;
    }
    if is_agent_active() || !std::io::stdin().is_terminal() {
        let mut details = serde_json::Map::new();
        details.insert(
            "path".into(),
            serde_json::Value::String(target.display().to_string()),
        );
        return Decision::Block(ErrorEnvelope::user_error_with_reason(
            format!("file exists: {}", target.display()),
            "setup_exists_non_interactive",
            details,
            i18n::t(keys::ERROR_SETUP_EXISTS_NON_INTERACTIVE),
        ));
    }
    let prompt = i18n::tf(
        keys::SETUP_OVERWRITE_CONFIRM,
        &[("path", &target.display().to_string())],
    );
    match Confirm::new(&prompt).with_default(false).prompt() {
        Ok(true) => Decision::Proceed,
        Ok(false) => Decision::Skip,
        Err(_) => Decision::Skip,
    }
}

fn write_atomic(target: &Path, bytes: Vec<u8>) -> Result<(), ErrorEnvelope> {
    let write_plan = plan(
        &[WriteTarget {
            path: PathBuf::from(target),
            content: bytes,
        }],
        OverwriteContext {
            policy: crate::core::config::OverwritePolicy::Always,
            force: true,
        },
    )?;
    commit(write_plan)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]
    use super::*;
    use crate::core::config::{
        Backend, EnabledTypes, Frontend, OverwritePolicy, SetupConfig, SetupSelections,
        SetupUserConfig, UserSelections,
    };

    #[test]
    fn project_config_roundtrip_has_no_overwrite() {
        let cfg = SetupConfig::from_selections(SetupSelections {
            backend: Backend::Java,
            frontend: Frontend::Vue,
            template: None,
        });
        let toml = cfg.to_toml_pretty().expect("serialize");
        assert!(!toml.contains("[overwrite]"));
    }

    #[test]
    fn user_config_serializes_required_sections() {
        let cfg = SetupUserConfig::from_user_selections(UserSelections {
            name: "Alice".into(),
            email: "a@example.com".into(),
            overwrite_policy: OverwritePolicy::ForceOnly,
            enabled_types: EnabledTypes::Backend,
        });
        let toml = cfg.to_toml_pretty().expect("serialize");
        assert!(toml.contains("[user]"));
        assert!(toml.contains("[overwrite]"));
        assert!(toml.contains("enabled-types = \"backend\""));
    }
}
