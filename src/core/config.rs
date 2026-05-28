//! Canonical `SetupConfig` (project) + `SetupUserConfig` (per-developer).
//!
//! Project schema lives in `.crud/setup.toml` (checked in); user schema lives
//! in `.crud/setup.user.toml` (gitignored). Overwrite policy and enabled
//! template scope are user-level — they must not bleed into shared config.

use serde::{Deserialize, Serialize};
use std::path::Path;

use std::collections::BTreeMap;

use super::default_paths::paths_for_frameworks;
use super::error::ErrorEnvelope;
use super::field_dsl::RESERVED_VARIABLE_NAMES;
use super::type_map::Fallback;

/// Parses the `--type-map-fallback` flag value mirroring `Fallback` deserialization:
/// `passthrough` → Passthrough, `error` → Error, anything else → Literal(s).
#[must_use]
pub fn parse_type_map_fallback(s: &str) -> Fallback {
    match s {
        "passthrough" => Fallback::Passthrough,
        "error" => Fallback::Error,
        other => Fallback::Literal(other.to_string()),
    }
}

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

/// Overwrite policy (user-level after split).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum OverwritePolicy {
    Never,
    #[serde(rename = "force-only")]
    ForceOnly,
    Always,
}

/// Which template subset to render unless `--type` overrides.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum EnabledTypes {
    All,
    Backend,
    Frontend,
}

impl EnabledTypes {
    fn default_value() -> Self {
        Self::All
    }
}

fn is_default_enabled_types(v: &EnabledTypes) -> bool {
    matches!(v, EnabledTypes::All)
}

/// Project wizard answers (no overwrite policy — that moved to user).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SetupSelections {
    pub backend: Backend,
    pub frontend: Frontend,
    pub component_library: ComponentLibrary,
}

/// User wizard answers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserSelections {
    pub name: String,
    pub email: String,
    pub overwrite_policy: OverwritePolicy,
    pub enabled_types: EnabledTypes,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PathsSection {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub java_base: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resources_base: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub doc_base: Option<String>,
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct UserSection {
    pub name: String,
    pub email: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ScopeSection {
    #[serde(
        rename = "enabled-types",
        default = "EnabledTypes::default_value",
        skip_serializing_if = "is_default_enabled_types"
    )]
    pub enabled_types: EnabledTypes,
}

impl Default for ScopeSection {
    fn default() -> Self {
        Self {
            enabled_types: EnabledTypes::default_value(),
        }
    }
}

/// Free-form `[variables]` table (D-G27).
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(transparent)]
pub struct VariablesSection(pub BTreeMap<String, toml::Value>);

/// `[templates.outputs]` keyed on template `rel_path` (D-G28 layer 2).
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(transparent)]
pub struct OutputsSection(pub BTreeMap<String, String>);

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct TemplatesParent {
    #[serde(default, skip_serializing_if = "is_empty_outputs")]
    pub outputs: OutputsSection,
}

fn is_empty_variables(s: &VariablesSection) -> bool {
    s.0.is_empty()
}

fn is_empty_outputs(s: &OutputsSection) -> bool {
    s.0.is_empty()
}

fn is_empty_templates_parent(t: &TemplatesParent) -> bool {
    t.outputs.0.is_empty()
}

/// `[type_map]` — global fallback policy for unknown types.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct TypeMapSection {
    #[serde(default)]
    pub fallback: Fallback,
}

fn is_default_type_map(t: &TypeMapSection) -> bool {
    matches!(t.fallback, Fallback::Passthrough)
}

/// Project setup.toml — shared / checked-in. Section order is contract (D-10).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct SetupConfig {
    pub project: ProjectSection,
    pub paths: PathsSection,
    #[serde(default, skip_serializing_if = "is_empty_variables")]
    pub variables: VariablesSection,
    #[serde(default, skip_serializing_if = "is_empty_templates_parent")]
    pub templates: TemplatesParent,
    #[serde(rename = "type_map", default, skip_serializing_if = "is_default_type_map")]
    pub type_map: TypeMapSection,
}

/// Per-developer setup.user.toml — gitignored.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SetupUserConfig {
    pub user: UserSection,
    pub overwrite: OverwriteSection,
    #[serde(default)]
    pub scope: ScopeSection,
}

/// Flag layer applied after defaults and optional file (CONF-04).
#[derive(Debug, Clone, Default)]
pub struct SetupFlagOverlay {
    pub backend: Option<Backend>,
    pub frontend: Option<Frontend>,
    pub component_library: Option<ComponentLibrary>,
    pub overwrite_policy: Option<OverwritePolicy>,
    pub enabled_types: Option<EnabledTypes>,
    pub type_map_fallback: Option<Fallback>,
}

impl SetupConfig {
    /// Default project selections when no file or flags are present.
    #[must_use]
    pub fn default_selections() -> SetupSelections {
        SetupSelections {
            backend: Backend::None,
            frontend: Frontend::None,
            component_library: ComponentLibrary::None,
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
            variables: VariablesSection::default(),
            templates: TemplatesParent::default(),
            type_map: TypeMapSection::default(),
        }
    }

    /// Merge precedence: defaults ← file ← flags for project fields.
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
        let mut cfg = Self::from_selections(sel);
        if let Some(fb) = flags.type_map_fallback {
            cfg.type_map.fallback = fb;
        }
        cfg
    }

    /// Deterministic TOML bytes for `.crud/setup.toml` (D-10).
    pub fn to_toml_pretty(&self) -> Result<String, ErrorEnvelope> {
        toml::to_string_pretty(self).map_err(|e| config_error(format!("serialize setup: {e}")))
    }
}

impl SetupUserConfig {
    #[must_use]
    pub fn default_user_selections() -> UserSelections {
        UserSelections {
            name: String::new(),
            email: String::new(),
            overwrite_policy: OverwritePolicy::Never,
            enabled_types: EnabledTypes::All,
        }
    }

    #[must_use]
    pub fn from_user_selections(selections: UserSelections) -> Self {
        Self {
            user: UserSection {
                name: selections.name,
                email: selections.email,
            },
            overwrite: OverwriteSection {
                overwrite_policy: selections.overwrite_policy,
            },
            scope: ScopeSection {
                enabled_types: selections.enabled_types,
            },
        }
    }

    /// Merge user file with flag overlay (flags win).
    pub fn merge_user(
        defaults: UserSelections,
        file: Option<&SetupUserConfig>,
        flags: SetupFlagOverlay,
    ) -> Self {
        let mut sel = defaults;
        if let Some(cfg) = file {
            sel.name = cfg.user.name.clone();
            sel.email = cfg.user.email.clone();
            sel.overwrite_policy = cfg.overwrite.overwrite_policy;
            sel.enabled_types = cfg.scope.enabled_types;
        }
        if let Some(v) = flags.overwrite_policy {
            sel.overwrite_policy = v;
        }
        if let Some(v) = flags.enabled_types {
            sel.enabled_types = v;
        }
        Self::from_user_selections(sel)
    }

    pub fn to_toml_pretty(&self) -> Result<String, ErrorEnvelope> {
        toml::to_string_pretty(self).map_err(|e| config_error(format!("serialize user setup: {e}")))
    }
}

/// Combined runtime view consumed by gen/validate.
#[derive(Debug, Clone)]
pub struct RuntimeConfig {
    pub project: SetupConfig,
    pub user: SetupUserConfig,
}

impl RuntimeConfig {
    /// Loads project + optional user config. Missing user file → defaults.
    pub fn load(project_path: &Path, user_path: &Path) -> Result<Self, ErrorEnvelope> {
        let project = load_setup_file(project_path)?;
        let user = if user_path.exists() {
            load_user_setup_file(user_path)?
        } else {
            SetupUserConfig::from_user_selections(SetupUserConfig::default_user_selections())
        };
        Ok(Self { project, user })
    }

    pub fn overwrite_policy(&self) -> OverwritePolicy {
        self.user.overwrite.overwrite_policy
    }

    pub fn enabled_types(&self) -> EnabledTypes {
        self.user.scope.enabled_types
    }
}

/// Parses an on-disk project setup file with unknown-field rejection (CONF-03).
pub fn load_setup_file(path: &Path) -> Result<SetupConfig, ErrorEnvelope> {
    let raw = std::fs::read_to_string(path).map_err(|e| {
        config_error(format!("read {}: {e}", path.display()))
    })?;
    let config: SetupConfig =
        toml::from_str(&raw).map_err(|e| config_error(format!("parse setup: {e}")))?;
    for key in config.variables.0.keys() {
        if RESERVED_VARIABLE_NAMES.contains(&key.as_str()) {
            let mut details = serde_json::Map::new();
            details.insert("variable".into(), serde_json::Value::String(key.clone()));
            return Err(ErrorEnvelope::config_error_with_reason(
                format!("reserved variable name: {key}"),
                "reserved_variable",
                details,
                "rename the variable; reserved: model, table, package, package_path, fields, model_*",
            ));
        }
    }
    Ok(config)
}

/// Parses an on-disk user setup file with unknown-field rejection.
pub fn load_user_setup_file(path: &Path) -> Result<SetupUserConfig, ErrorEnvelope> {
    let raw = std::fs::read_to_string(path).map_err(|e| {
        config_error(format!("read {}: {e}", path.display()))
    })?;
    let config: SetupUserConfig =
        toml::from_str(&raw).map_err(|e| config_error(format!("parse user setup: {e}")))?;
    if config.user.name.trim().is_empty() {
        return Err(config_error("user.name must not be empty"));
    }
    if config.user.email.trim().is_empty() {
        return Err(config_error("user.email must not be empty"));
    }
    Ok(config)
}

fn config_error(msg: impl Into<String>) -> ErrorEnvelope {
    ErrorEnvelope::config_error_with_reason(msg, "config_error", serde_json::Map::new(), "")
}
