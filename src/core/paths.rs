//! Cross-platform path helpers (D-16, CONF-09).

use std::path::PathBuf;

use super::error::{ErrorEnvelope, Kind};

/// Resolves the global CRUD directory under the user home (`~/.crud`).
pub fn global_crud_dir() -> Result<PathBuf, ErrorEnvelope> {
    let home = dirs::home_dir().ok_or_else(|| {
        ErrorEnvelope {
            kind: Kind::ConfigError,
            msg: "home directory not found".into(),
            exit_code: Kind::ConfigError.exit_code(),
            hint: "set HOME or USERPROFILE".into(),
            details: serde_json::Map::new(),
        }
    })?;
    Ok(home.join(".crud"))
}

/// Project-local setup file relative to the given project root.
#[must_use]
pub fn project_setup_toml(project_root: &std::path::Path) -> PathBuf {
    project_root.join(".crud").join("setup.toml")
}
