//! Project cwd and template-root resolution for MCP handlers.

use std::path::{Path, PathBuf};

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

/**
 * Reads a file under `templates_root` if it exists.
 */
pub fn read_bundle_file(templates_root: &Path, name: &str) -> Result<String, ErrorEnvelope> {
    let path = templates_root.join(name);
    if !path.exists() {
        return Ok(String::new());
    }
    std::fs::read_to_string(&path).map_err(|e| {
        ErrorEnvelope::template_error(format!("read {}: {e}", path.display()))
    })
}
