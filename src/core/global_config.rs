//! Global per-user CLI preferences stored at `~/.crud/config.toml`.
//!
//! Currently holds only the UI language preference. This file is user-global
//! (not per project) and is never checked into a repository.

use serde::{Deserialize, Serialize};
use std::path::Path;

use super::error::{ErrorEnvelope, Kind};
use super::i18n::{self, keys, Lang};

/// `[ui]` section of the global config.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct UiSection {
    /// Preferred UI language code (`en` / `zh`); absent until first chosen.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lang: Option<String>,
}

impl UiSection {
    fn is_empty(&self) -> bool {
        self.lang.is_none()
    }
}

/// `[templates]` section: template-install registry preferences.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct TemplatesRegistrySection {
    /// GitHub `owner/repo` (or full `https://github.com/owner/repo` URL) that
    /// `crud-cli template install` downloads from when no `--repo` flag is set.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repo: Option<String>,
}

impl TemplatesRegistrySection {
    fn is_empty(&self) -> bool {
        self.repo.is_none()
    }
}

/// Default GitHub repository used by `template install` when neither the
/// `--repo` flag nor `[templates].repo` is set.
pub const DEFAULT_TEMPLATE_REPO: &str = "ttTennessee/crud-templates";

/// Root schema for `~/.crud/config.toml`.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct GlobalConfig {
    #[serde(default, skip_serializing_if = "UiSection::is_empty")]
    pub ui: UiSection,
    #[serde(default, skip_serializing_if = "TemplatesRegistrySection::is_empty")]
    pub templates: TemplatesRegistrySection,
}

impl GlobalConfig {
    /// Loads the config, returning defaults when the file is absent.
    pub fn load(path: &Path) -> Result<Self, ErrorEnvelope> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let raw = std::fs::read_to_string(path)
            .map_err(|e| config_error(format!("read {}: {e}", path.display())))?;
        toml::from_str(&raw).map_err(|e| config_error(format!("parse global config: {e}")))
    }

    /// Best-effort load that never fails (falls back to defaults on any error).
    #[must_use]
    pub fn load_or_default(path: &Path) -> Self {
        Self::load(path).unwrap_or_default()
    }

    /// Resolves the stored language preference, if any and recognized.
    #[must_use]
    pub fn lang(&self) -> Option<Lang> {
        self.ui.lang.as_deref().and_then(Lang::parse)
    }

    /// Records the chosen language preference.
    pub fn set_lang(&mut self, lang: Lang) {
        self.ui.lang = Some(lang.code().to_string());
    }

    /// Configured template repo, falling back to [`DEFAULT_TEMPLATE_REPO`].
    #[must_use]
    pub fn template_repo(&self) -> &str {
        self.templates
            .repo
            .as_deref()
            .unwrap_or(DEFAULT_TEMPLATE_REPO)
    }

    /// Serializes to deterministic TOML bytes.
    pub fn to_toml_pretty(&self) -> Result<String, ErrorEnvelope> {
        toml::to_string_pretty(self)
            .map_err(|e| config_error(format!("serialize global config: {e}")))
    }

    /// Atomically writes the config, creating `~/.crud` if needed.
    pub fn save(&self, path: &Path) -> Result<(), ErrorEnvelope> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| config_error(format!("create {}: {e}", parent.display())))?;
        }
        let body = self.to_toml_pretty()?;
        std::fs::write(path, body.as_bytes())
            .map_err(|e| config_error(format!("write {}: {e}", path.display())))
    }
}

/// Environment override variable for the UI language.
pub const LANG_ENV: &str = "CRUD_LANG";

/// Resolves the preferred language without prompting.
///
/// Precedence: `CRUD_LANG` env var → stored global config → `None` (caller may
/// then prompt, or fall back to the default English locale).
#[must_use]
pub fn resolve_preferred_lang(cfg: &GlobalConfig) -> Option<Lang> {
    if let Ok(raw) = std::env::var(LANG_ENV) {
        if let Some(lang) = Lang::parse(&raw) {
            return Some(lang);
        }
    }
    cfg.lang()
}

/// True when `CRUD_LANG` is set to a recognized value (env wins, no prompt).
#[must_use]
pub fn lang_env_override() -> Option<Lang> {
    std::env::var(LANG_ENV).ok().and_then(|v| Lang::parse(&v))
}

fn config_error(msg: impl Into<String>) -> ErrorEnvelope {
    ErrorEnvelope {
        kind: Kind::ConfigError,
        msg: msg.into(),
        exit_code: Kind::ConfigError.exit_code(),
        hint: i18n::t(keys::ERROR_GLOBAL_CONFIG_CHECK).into(),
        details: serde_json::Map::new(),
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn load_missing_returns_default() {
        let dir = TempDir::new().expect("tmp");
        let path = dir.path().join("config.toml");
        let cfg = GlobalConfig::load(&path).expect("ok");
        assert_eq!(cfg, GlobalConfig::default());
        assert_eq!(cfg.lang(), None);
    }

    #[test]
    fn round_trips_lang() {
        let dir = TempDir::new().expect("tmp");
        let path = dir.path().join(".crud").join("config.toml");
        let mut cfg = GlobalConfig::default();
        cfg.set_lang(Lang::Zh);
        cfg.save(&path).expect("save");
        let loaded = GlobalConfig::load(&path).expect("load");
        assert_eq!(loaded.lang(), Some(Lang::Zh));
    }
}
