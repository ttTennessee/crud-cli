//! `crud-cli setup` execution pipeline (D-10, CONF-08, FOUND-09).

use std::env;
use std::path::PathBuf;

use crate::core::config::SetupConfig;
use crate::core::error::ErrorEnvelope;
use crate::core::fs_writer::{commit, plan, OverwriteContext, WriteTarget};
use crate::core::paths::project_setup_toml;

use crate::cli::args::{exit_with_envelope, SetupArgs};
use crate::cli::output::emit_success;
use crate::cli::setup_wizard::run_interactive_wizard;

/// Runs setup end-to-end: build config, preflight, atomic write.
pub fn run_setup(setup: SetupArgs) -> i32 {
    let config_result = if setup.is_non_interactive() {
        setup.to_setup_config()
    } else {
        run_interactive_wizard()
    };

    match config_result {
        Ok(cfg) => match write_setup_config(&cfg, setup.force) {
            Ok(path) => {
                let human = format!("Created {}", path.display());
                emit_success(Some(&human));
                0
            }
            Err(envelope) => exit_with_envelope(&envelope),
        },
        Err(envelope) => exit_with_envelope(&envelope),
    }
}

fn write_setup_config(config: &SetupConfig, force: bool) -> Result<PathBuf, ErrorEnvelope> {
    let project_root = env::current_dir().map_err(|e| {
        ErrorEnvelope {
            kind: crate::core::error::Kind::ConfigError,
            msg: format!("resolve project root: {e}"),
            exit_code: crate::core::error::Kind::ConfigError.exit_code(),
            hint: String::new(),
            details: serde_json::Map::new(),
        }
    })?;
    let target = project_setup_toml(&project_root);
    let content = config.to_toml_pretty()?;
    let overwrite = OverwriteContext {
        policy: config.overwrite.overwrite_policy,
        force,
    };
    let write_plan = plan(
        &[WriteTarget {
            path: target.clone(),
            content: content.into_bytes(),
        }],
        overwrite,
    )?;
    commit(write_plan)?;
    Ok(target)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::config::{Backend, ComponentLibrary, Frontend, OverwritePolicy, SetupSelections};

    #[test]
    fn overwrite_context_from_config() {
        let cfg = SetupConfig::from_selections(SetupSelections {
            backend: Backend::None,
            frontend: Frontend::None,
            component_library: ComponentLibrary::None,
            overwrite_policy: OverwritePolicy::ForceOnly,
        });
        assert_eq!(cfg.overwrite.overwrite_policy, OverwritePolicy::ForceOnly);
    }
}
