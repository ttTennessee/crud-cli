//! Canonical `SetupConfig` and merge / serialization (D-10, CONF-03..05).

use serde::{Deserialize, Serialize};
use std::path::Path;

use super::default_paths::paths_for_frameworks;
use super::error::{ErrorEnvelope, Kind};

/// Closed-set backend (D-08).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Backend {
    #[serde(rename = "spring-boot")]
    SpringBoot,
    Nest,
    None,
}

/// Closed-set frontend (D-08).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Frontend {
    Vue,
    React,
    None,
}

/// Closed-set component library (D-08).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ComponentLibrary {
    #[serde(rename = "element-plus")]
    ElementPlus,
    Antd,
    #[serde(rename = "naive-ui")]
    NaiveUi,
    None,
}

/// Overwrite policy (D-08 / CONF-08).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum OverwritePolicy {
    Never,
    #[serde(rename = "force-only")]
    ForceOnly,
    Always,
}

/// User-facing selections for both flag and wizard paths (D-10).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SetupSelections {
    pub backend: Backend,
    pub frontend: Frontend,
    pub component_library: ComponentLibrary,
    pub overwrite_policy: OverwritePolicy,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PathsSection {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub java_base: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nest_base: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vue_base: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub react_base: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ProjectSection {
    pub backend: Backend,
    pub frontend: Frontend,
    #[serde(rename = "component-library")]
    pub component_library: ComponentLibrary,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct OverwriteSection {
    #[serde(rename = "overwrite-policy")]
    pub overwrite_policy: OverwritePolicy,
}

/// Canonical setup.toml root — section order is contract (D-10).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SetupConfig {
    pub project: ProjectSection,
    pub paths: PathsSection,
    pub overwrite: OverwriteSection,
}

/// Flag layer applied after defaults and optional file (CONF-04).
#[derive(Debug, Clone, Copy, Default)]
pub struct SetupFlagOverlay {
    pub backend: Option<Backend>,
    pub frontend: Option<Frontend>,
    pub component_library: Option<ComponentLibrary>,
    pub overwrite_policy: Option<OverwritePolicy>,
}

impl SetupConfig {
    /// Default selections when no file or flags are present.
    #[must_use]
    pub fn default_selections() -> SetupSelections {
        SetupSelections {
            backend: Backend::None,
            frontend: Frontend::None,
            component_library: ComponentLibrary::None,
            overwrite_policy: OverwritePolicy::Never,
        }
    }

    /// Single builder for interactive and non-interactive inputs (D-10).
    #[must_use]
    pub fn from_selections(selections: SetupSelections) -> Self {
        Self {
            project: ProjectSection {
                backend: selections.backend,
                frontend: selections.frontend,
                component_library: selections.component_library,
            },
            paths: paths_for_frameworks(selections.backend, selections.frontend),
            overwrite: OverwriteSection {
                overwrite_policy: selections.overwrite_policy,
            },
        }
    }

    /// Merge precedence: defaults ← file ← flags (CONF-04).
    pub fn merge(
        defaults: SetupSelections,
        file: Option<&SetupConfig>,
        flags: SetupFlagOverlay,
    ) -> Self {
        let mut sel = defaults;
        if let Some(cfg) = file {
            sel.backend = cfg.project.backend;
            sel.frontend = cfg.project.frontend;
            sel.component_library = cfg.project.component_library;
            sel.overwrite_policy = cfg.overwrite.overwrite_policy;
        }
        if let Some(v) = flags.backend {
            sel.backend = v;
        }
        if let Some(v) = flags.frontend {
            sel.frontend = v;
        }
        if let Some(v) = flags.component_library {
            sel.component_library = v;
        }
        if let Some(v) = flags.overwrite_policy {
            sel.overwrite_policy = v;
        }
        Self::from_selections(sel)
    }

    /// Deterministic TOML bytes for `.crud/setup.toml` (D-10).
    pub fn to_toml_pretty(&self) -> Result<String, ErrorEnvelope> {
        toml::to_string_pretty(self).map_err(|e| config_error(format!("serialize setup: {e}")))
    }
}

/// Parses an on-disk setup file with unknown-field rejection (CONF-03).
pub fn load_setup_file(path: &Path) -> Result<SetupConfig, ErrorEnvelope> {
    let raw = std::fs::read_to_string(path).map_err(|e| {
        config_error(format!("read {}: {e}", path.display()))
    })?;
    toml::from_str(&raw).map_err(|e| config_error(format!("parse setup: {e}")))
}

fn config_error(msg: impl Into<String>) -> ErrorEnvelope {
    ErrorEnvelope {
        kind: Kind::ConfigError,
        msg: msg.into(),
        exit_code: Kind::ConfigError.exit_code(),
        hint: String::new(),
        details: serde_json::Map::new(),
    }
}
