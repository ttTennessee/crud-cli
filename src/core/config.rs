//! Canonical `SetupConfig` (project) + `SetupUserConfig` (per-developer).
//!
//! Project schema lives in `.crud/setup.toml` (checked in); user schema lives
//! in `.crud/setup.user.toml` (gitignored). Overwrite policy and enabled
//! template scope are user-level — they must not bleed into shared config.

use serde::{de::Error as DeError, Deserialize, Deserializer, Serialize, Serializer};
use std::collections::BTreeMap;
use std::fmt;
use std::path::Path;
use std::str::FromStr;

use super::default_paths::paths_for_selections;
use super::error::ErrorEnvelope;
use super::field_dsl::RESERVED_VARIABLE_NAMES;
use super::i18n::{self, keys};
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

/// Backend language. Open-ended via `Custom` so any language declared by a
/// template manifest or chosen via the wizard's "custom" option is preserved.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Backend {
    Java,
    TypeScript,
    Go,
    Python,
    None,
    Custom(String),
}

/// Frontend framework / language.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Frontend {
    Vue,
    React,
    None,
    Custom(String),
}

impl Backend {
    /// Canonical lowercase identifier (also the TOML serialized form and the
    /// key under `[paths.lang]`).
    #[must_use]
    pub fn as_key(&self) -> &str {
        match self {
            Self::Java => "java",
            Self::TypeScript => "typescript",
            Self::Go => "go",
            Self::Python => "python",
            Self::None => "none",
            Self::Custom(s) => s.as_str(),
        }
    }

    /// Parses a lowercase identifier into a known variant or `Custom`.
    /// Empty strings are rejected.
    pub fn parse(value: &str) -> Result<Self, &'static str> {
        let v = value.trim();
        if v.is_empty() {
            return Err("backend value must not be empty");
        }
        Ok(match v {
            "java" => Self::Java,
            "typescript" | "ts" => Self::TypeScript,
            "go" => Self::Go,
            "python" | "py" => Self::Python,
            "none" => Self::None,
            other if is_valid_lang_id(other) => Self::Custom(other.to_string()),
            _ => return Err("backend must match [a-z0-9][a-z0-9-]*"),
        })
    }

    /// True iff this backend is `None`.
    #[must_use]
    pub fn is_none(&self) -> bool {
        matches!(self, Self::None)
    }
}

impl Frontend {
    #[must_use]
    pub fn as_key(&self) -> &str {
        match self {
            Self::Vue => "vue",
            Self::React => "react",
            Self::None => "none",
            Self::Custom(s) => s.as_str(),
        }
    }

    pub fn parse(value: &str) -> Result<Self, &'static str> {
        let v = value.trim();
        if v.is_empty() {
            return Err("frontend value must not be empty");
        }
        Ok(match v {
            "vue" => Self::Vue,
            "react" => Self::React,
            "none" => Self::None,
            other if is_valid_lang_id(other) => Self::Custom(other.to_string()),
            _ => return Err("frontend must match [a-z0-9][a-z0-9-]*"),
        })
    }

    #[must_use]
    pub fn is_none(&self) -> bool {
        matches!(self, Self::None)
    }
}

/// Allowed characters for a custom language identifier — keeps `paths.lang.<key>`
/// stable under TOML and predictable in template path joins.
#[must_use]
pub fn is_valid_lang_id(s: &str) -> bool {
    if s.is_empty() {
        return false;
    }
    let mut chars = s.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !first.is_ascii_lowercase() && !first.is_ascii_digit() {
        return false;
    }
    chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

impl Serialize for Backend {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(self.as_key())
    }
}

impl<'de> Deserialize<'de> for Backend {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let s = String::deserialize(d)?;
        Backend::parse(&s).map_err(D::Error::custom)
    }
}

impl Serialize for Frontend {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(self.as_key())
    }
}

impl<'de> Deserialize<'de> for Frontend {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let s = String::deserialize(d)?;
        Frontend::parse(&s).map_err(D::Error::custom)
    }
}

/// `[project].template = "name[@version]"` reference.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TemplateRef {
    pub name: String,
    pub version: Option<String>,
}

impl TemplateRef {
    pub fn parse(input: &str) -> Result<Self, &'static str> {
        let trimmed = input.trim();
        if trimmed.is_empty() {
            return Err("template ref must not be empty");
        }
        let (name, version) = match trimmed.rsplit_once('@') {
            Some((n, v)) => (n, Some(v.to_string())),
            None => (trimmed, None),
        };
        if name.is_empty() {
            return Err("template name must not be empty");
        }
        Ok(Self {
            name: name.to_string(),
            version,
        })
    }
}

impl fmt::Display for TemplateRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.version {
            Some(v) => write!(f, "{}@{}", self.name, v),
            None => write!(f, "{}", self.name),
        }
    }
}

impl FromStr for TemplateRef {
    type Err = &'static str;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s)
    }
}

impl Serialize for TemplateRef {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for TemplateRef {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let s = String::deserialize(d)?;
        TemplateRef::parse(&s).map_err(D::Error::custom)
    }
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

/// Project wizard answers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SetupSelections {
    pub backend: Backend,
    pub frontend: Frontend,
    pub template: Option<TemplateRef>,
}

/// User wizard answers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserSelections {
    pub name: String,
    pub email: String,
    pub overwrite_policy: OverwritePolicy,
    pub enabled_types: EnabledTypes,
}

/// `[paths]` is now two maps: one keyed by language identifier (matching the
/// template subdirectory prefix and `Backend/Frontend::as_key()`), one for
/// auxiliary shared roots like `resources` and `doc`.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PathsSection {
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub lang: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub aux: BTreeMap<String, String>,
}

impl PathsSection {
    /// Looks up a path for the given bundle prefix (`"java"`, `"vue"`,
    /// `"resources"`, etc.), checking `lang` first, then `aux`.
    #[must_use]
    pub fn lookup(&self, key: &str) -> Option<&str> {
        self.lang
            .get(key)
            .or_else(|| self.aux.get(key))
            .map(String::as_str)
    }

    /// Bundle prefixes enabled by the project (union of lang + aux keys).
    /// Used by `gen` to default the `--type` filter and by `validate` to know
    /// which subdirectories to scan.
    pub fn enabled_prefixes(&self) -> Vec<String> {
        let mut out: Vec<String> = self
            .lang
            .keys()
            .chain(self.aux.keys())
            .cloned()
            .collect();
        out.sort();
        out.dedup();
        out
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ProjectSection {
    pub backend: Backend,
    pub frontend: Frontend,
    /// When set, `gen` reads templates from `~/.crud/templates/<name>/<version>/`
    /// instead of the project-local `.crud/templates/`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub template: Option<TemplateRef>,
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

/// Free-form `[variables]` table.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(transparent)]
pub struct VariablesSection(pub BTreeMap<String, toml::Value>);

/// `[templates.outputs]` keyed on template `rel_path`.
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

/// Project setup.toml — shared / checked-in. Section order is contract .
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct SetupConfig {
    pub project: ProjectSection,
    #[serde(default)]
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

/// Flag layer applied after defaults and optional file.
#[derive(Debug, Clone, Default)]
pub struct SetupFlagOverlay {
    pub backend: Option<Backend>,
    pub frontend: Option<Frontend>,
    pub template: Option<TemplateRef>,
    pub overwrite_policy: Option<OverwritePolicy>,
    pub enabled_types: Option<EnabledTypes>,
    pub type_map_fallback: Option<Fallback>,
    /// `--lang key=path` repeats merged into `[paths.lang]`.
    pub paths_lang: BTreeMap<String, String>,
    /// `--aux key=path` repeats merged into `[paths.aux]`.
    pub paths_aux: BTreeMap<String, String>,
}

impl SetupConfig {
    /// Default project selections when no file or flags are present.
    #[must_use]
    pub fn default_selections() -> SetupSelections {
        SetupSelections {
            backend: Backend::None,
            frontend: Frontend::None,
            template: None,
        }
    }

    /// Single builder for interactive and non-interactive inputs.
    #[must_use]
    pub fn from_selections(selections: SetupSelections) -> Self {
        let paths = paths_for_selections(&selections.backend, &selections.frontend);
        Self {
            project: ProjectSection {
                backend: selections.backend,
                frontend: selections.frontend,
                template: selections.template,
            },
            paths,
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
            sel.backend = cfg.project.backend.clone();
            sel.frontend = cfg.project.frontend.clone();
            sel.template = cfg.project.template.clone();
        }
        if let Some(v) = flags.backend {
            sel.backend = v;
        }
        if let Some(v) = flags.frontend {
            sel.frontend = v;
        }
        if let Some(v) = flags.template {
            sel.template = Some(v);
        }
        let mut cfg = Self::from_selections(sel);
        for (k, v) in flags.paths_lang {
            cfg.paths.lang.insert(k, v);
        }
        for (k, v) in flags.paths_aux {
            cfg.paths.aux.insert(k, v);
        }
        if let Some(fb) = flags.type_map_fallback {
            cfg.type_map.fallback = fb;
        }
        cfg
    }

    /// Deterministic TOML bytes for `.crud/setup.toml`.
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

/// Pre-checks for legacy schema fields (pre-language-rename). Returns a clear
/// error directing the user to rerun `crud-cli setup` rather than letting the
/// strict deserializer emit an opaque `unknown field` message.
fn detect_legacy_schema(raw: &str) -> Result<(), ErrorEnvelope> {
    let parsed: toml::Value = match toml::from_str(raw) {
        Ok(v) => v,
        Err(_) => return Ok(()), // strict deserializer will report a better error
    };
    let mut legacy: Vec<&'static str> = Vec::new();
    if let Some(project) = parsed.get("project").and_then(|v| v.as_table()) {
        if project.contains_key("component-library") || project.contains_key("component_library") {
            legacy.push("project.component-library");
        }
        if let Some(backend) = project.get("backend").and_then(|v| v.as_str()) {
            if backend == "spring-boot" || backend == "nest" {
                legacy.push("project.backend (framework name)");
            }
        }
    }
    if let Some(paths) = parsed.get("paths").and_then(|v| v.as_table()) {
        for legacy_key in [
            "java_base",
            "resources_base",
            "doc_base",
            "nest_base",
            "vue_base",
            "react_base",
        ] {
            if paths.contains_key(legacy_key) {
                legacy.push("paths.*_base");
                break;
            }
        }
    }
    if legacy.is_empty() {
        return Ok(());
    }
    let mut details = serde_json::Map::new();
    details.insert(
        "legacy_fields".into(),
        serde_json::Value::Array(
            legacy
                .iter()
                .map(|s| serde_json::Value::String((*s).to_string()))
                .collect(),
        ),
    );
    Err(ErrorEnvelope::config_error_with_reason(
        format!("legacy setup.toml schema (fields: {})", legacy.join(", ")),
        "legacy_schema",
        details,
        i18n::t(keys::ERROR_CONFIG_LEGACY_SCHEMA),
    ))
}

/// Parses an on-disk project setup file with unknown-field rejection.
pub fn load_setup_file(path: &Path) -> Result<SetupConfig, ErrorEnvelope> {
    let raw = std::fs::read_to_string(path).map_err(|e| {
        config_error(format!("read {}: {e}", path.display()))
    })?;
    detect_legacy_schema(&raw)?;
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
                i18n::t(keys::ERROR_CONFIG_RESERVED_VARIABLE),
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

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]
    use super::*;

    #[test]
    fn backend_round_trip_known() {
        for b in [Backend::Java, Backend::TypeScript, Backend::Go, Backend::Python, Backend::None] {
            let toml_str = toml::to_string(&Wrap { v: b.clone() }).expect("serialize");
            let decoded: Wrap<Backend> = toml::from_str(&toml_str).expect("deserialize");
            assert_eq!(decoded.v, b);
        }
    }

    #[test]
    fn backend_custom_value() {
        let toml_str = "v = \"php\"";
        let decoded: Wrap<Backend> = toml::from_str(toml_str).expect("deserialize");
        assert_eq!(decoded.v, Backend::Custom("php".to_string()));
    }

    #[test]
    fn backend_rejects_invalid_identifier() {
        let toml_str = "v = \"Spring Boot\"";
        let err = toml::from_str::<Wrap<Backend>>(toml_str).expect_err("should reject");
        assert!(format!("{err}").contains("backend"));
    }

    #[test]
    fn template_ref_round_trip() {
        let with_v = TemplateRef::parse("ruoyi@1.0.0").expect("ok");
        assert_eq!(with_v.name, "ruoyi");
        assert_eq!(with_v.version.as_deref(), Some("1.0.0"));
        assert_eq!(with_v.to_string(), "ruoyi@1.0.0");

        let no_v = TemplateRef::parse("eladmin").expect("ok");
        assert_eq!(no_v.version, None);
        assert_eq!(no_v.to_string(), "eladmin");
    }

    #[test]
    fn paths_section_lookup_prefers_lang() {
        let mut s = PathsSection::default();
        s.lang.insert("java".into(), "backend".into());
        s.aux.insert("doc".into(), "docs".into());
        assert_eq!(s.lookup("java"), Some("backend"));
        assert_eq!(s.lookup("doc"), Some("docs"));
        assert_eq!(s.lookup("missing"), None);
    }

    #[test]
    fn legacy_schema_detected() {
        let raw = r#"
[project]
backend = "spring-boot"
frontend = "vue"
component-library = "element-plus"
"#;
        let err = detect_legacy_schema(raw).expect_err("should reject");
        assert_eq!(err.kind, super::super::error::Kind::ConfigError);
    }

    #[test]
    fn legacy_paths_keys_detected() {
        let raw = r#"
[project]
backend = "java"
frontend = "vue"

[paths]
nest_base = "src"
"#;
        let err = detect_legacy_schema(raw).expect_err("should reject");
        assert_eq!(err.kind, super::super::error::Kind::ConfigError);
    }

    #[test]
    fn new_schema_round_trips_via_toml_pretty() {
        let mut sel = SetupConfig::default_selections();
        sel.backend = Backend::Java;
        sel.frontend = Frontend::Vue;
        sel.template = Some(TemplateRef::parse("ruoyi@1.0.0").expect("ok"));
        let cfg = SetupConfig::from_selections(sel.clone());
        let s = cfg.to_toml_pretty().expect("ok");
        let decoded: SetupConfig = toml::from_str(&s).expect("decode");
        assert_eq!(decoded.project.backend, Backend::Java);
        assert_eq!(decoded.project.frontend, Frontend::Vue);
        assert_eq!(
            decoded.project.template.as_ref().map(ToString::to_string),
            Some("ruoyi@1.0.0".to_string())
        );
    }

    /// Used to roundtrip the enums through TOML without a containing struct.
    #[derive(Debug, Serialize, Deserialize)]
    struct Wrap<T> {
        v: T,
    }
}
