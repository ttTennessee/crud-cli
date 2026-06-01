//! Cross-platform path helpers (D-16, CONF-09).

use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

use super::error::{ErrorEnvelope, Kind};
use super::i18n::{self, keys};

/// Resolves the global CRUD directory under the user home (`~/.crud`).
///
/// Honors `CRUD_HOME` as an explicit override (used by tests and CI to avoid
/// `dirs::home_dir()` quirks — notably Windows where the home is read from
/// `SHGetKnownFolderPath` and ignores env vars entirely).
pub fn global_crud_dir() -> Result<PathBuf, ErrorEnvelope> {
    if let Some(override_home) = std::env::var_os("CRUD_HOME") {
        let p = PathBuf::from(override_home);
        if !p.as_os_str().is_empty() {
            return Ok(p.join(".crud"));
        }
    }
    let home = dirs::home_dir().ok_or_else(|| {
        ErrorEnvelope {
            kind: Kind::ConfigError,
            msg: "home directory not found".into(),
            exit_code: Kind::ConfigError.exit_code(),
            hint: i18n::t(keys::ERROR_PATHS_HOME_NOT_FOUND).into(),
            details: serde_json::Map::new(),
        }
    })?;
    Ok(home.join(".crud"))
}

/// Global per-user CLI preferences file (`~/.crud/config.toml`).
pub fn global_config_toml() -> Result<PathBuf, ErrorEnvelope> {
    Ok(global_crud_dir()?.join("config.toml"))
}

/// Project-local setup file relative to the given project root.
#[must_use]
pub fn project_setup_toml(project_root: &Path) -> PathBuf {
    project_root.join(".crud").join("setup.toml")
}

/// Per-developer setup file (gitignored).
#[must_use]
pub fn project_setup_user_toml(project_root: &Path) -> PathBuf {
    project_root.join(".crud").join("setup.user.toml")
}

/// `.crud/.gitignore` controlling user-only files.
#[must_use]
pub fn project_crud_gitignore(project_root: &Path) -> PathBuf {
    project_root.join(".crud").join(".gitignore")
}

/// Append a single line to `.crud/.gitignore` iff not already present.
/// Idempotent; creates the file (and parent) on first call.
pub fn ensure_gitignore_entry(path: &Path, entry: &str) -> Result<(), ErrorEnvelope> {
    let parent = path
        .parent()
        .ok_or_else(|| gitignore_error(path, "missing parent directory"))?;
    std::fs::create_dir_all(parent)
        .map_err(|e| gitignore_error(path, format!("create dirs: {e}")))?;

    if path.exists() {
        let file = std::fs::File::open(path)
            .map_err(|e| gitignore_error(path, format!("open: {e}")))?;
        for line in BufReader::new(file).lines() {
            let line = line.map_err(|e| gitignore_error(path, format!("read: {e}")))?;
            if line.trim() == entry {
                return Ok(());
            }
        }
    }

    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|e| gitignore_error(path, format!("open append: {e}")))?;
    let needs_leading_newline = path
        .metadata()
        .map(|m| m.len() > 0)
        .unwrap_or(false)
        && !ends_with_newline(path).unwrap_or(true);
    let mut buf = String::new();
    if needs_leading_newline {
        buf.push('\n');
    }
    buf.push_str(entry);
    buf.push('\n');
    file.write_all(buf.as_bytes())
        .map_err(|e| gitignore_error(path, format!("write: {e}")))?;
    Ok(())
}

fn ends_with_newline(path: &Path) -> std::io::Result<bool> {
    let bytes = std::fs::read(path)?;
    Ok(bytes.last().is_some_and(|b| *b == b'\n'))
}

fn gitignore_error(path: &Path, msg: impl Into<String>) -> ErrorEnvelope {
    let mut details = serde_json::Map::new();
    details.insert(
        "path".into(),
        serde_json::Value::String(path.display().to_string()),
    );
    ErrorEnvelope {
        kind: Kind::ConfigError,
        msg: msg.into(),
        exit_code: Kind::ConfigError.exit_code(),
        hint: i18n::t(keys::ERROR_PATHS_GITIGNORE_WRITE).into(),
        details,
    }
}
