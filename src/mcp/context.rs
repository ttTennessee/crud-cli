//! Project cwd and template-root resolution for MCP handlers.

use std::path::PathBuf;

use crate::core::config::RuntimeConfig;
use crate::core::error::ErrorEnvelope;
use crate::core::paths::{project_setup_toml, project_setup_user_toml};
use crate::core::template_loader;

/**
 * Loaded project runtime: merged setup + resolved template bundle root.
 */
#[derive(Clone)]
pub struct ProjectContext {
    pub cwd: PathBuf,
    pub templates_root: PathBuf,
}

/**
 * Resolves `cwd` (defaults to process cwd) and the active template bundle root.
 */
pub fn load_project_context(cwd: Option<PathBuf>) -> Result<ProjectContext, ErrorEnvelope> {
    let cwd = cwd.unwrap_or_else(|| {
        std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
    });
    let runtime = RuntimeConfig::load(
        &project_setup_toml(&cwd),
        &project_setup_user_toml(&cwd),
    )?;
    let templates_root = template_loader::resolve_templates_root(&cwd, &runtime.project)?;
    Ok(ProjectContext {
        cwd,
        templates_root,
    })
}

