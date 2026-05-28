//! clap surface for `setup` (D-08, CONF-02, CONF-08, FOUND-04).

use clap::{Parser, Subcommand, ValueEnum};
use std::ffi::OsString;

use crate::core::config::{
    Backend, ComponentLibrary, EnabledTypes, Frontend, OverwritePolicy, SetupConfig,
    SetupFlagOverlay, SetupSelections, SetupUserConfig, UserSelections,
};
use crate::core::error::ErrorEnvelope;

use super::output::emit_failure;

/// Root CLI parser (FOUND-04).
#[derive(Parser, Debug)]
#[command(name = "crud-cli", version, about)]
pub struct Cli {
    /// Agent/machine mode: JSON errors on stderr, empty success stdout (D-05).
    #[arg(long, global = true)]
    pub agent: bool,

    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Create or refresh project / user setup configuration.
    Setup(SetupArgs),
    /// Generate CRUD files from project templates.
    Gen(GenArgs),
    /// Validate project templates before generation.
    Validate(ValidateArgs),
}

/// `crud-cli setup` flags. Default scope = user; `--project` switches target.
#[derive(Parser, Debug, Default)]
pub struct SetupArgs {
    /// Write the shared project config (`.crud/setup.toml`) instead of the
    /// per-developer user config (`.crud/setup.user.toml`).
    #[arg(long = "project", default_value_t = false)]
    pub project: bool,

    // Project-scope flags
    #[arg(long = "backend", value_enum)]
    pub backend: Option<SetupBackend>,

    #[arg(long = "frontend", value_enum)]
    pub frontend: Option<SetupFrontend>,

    #[arg(long = "component-library", value_enum)]
    pub component_library: Option<SetupComponentLibrary>,

    // User-scope flags
    #[arg(long = "overwrite-policy", value_enum)]
    pub overwrite_policy: Option<SetupOverwritePolicy>,

    #[arg(long = "enabled-types", value_enum)]
    pub enabled_types: Option<SetupEnabledTypes>,

    #[arg(long = "user-name")]
    pub user_name: Option<String>,

    #[arg(long = "user-email")]
    pub user_email: Option<String>,

    /// Overwrite the target file without the interactive confirm.
    #[arg(long = "force", default_value_t = false)]
    pub force: bool,
}

/// `crud-cli gen` flags (D-G01, D-G10, D-G11).
#[derive(Parser, Debug, Default)]
pub struct GenArgs {
    /// Entity / model name (positional).
    pub name: Option<String>,

    /// Micro-DSL field list (`name:Type`, comma-separated).
    #[arg(long = "fields")]
    pub fields: Option<String>,

    /// JSON entity definition file (mutually exclusive with `--fields`).
    #[arg(long = "file")]
    pub file: Option<std::path::PathBuf>,

    #[arg(long = "package")]
    pub package: Option<String>,

    #[arg(long = "table")]
    pub table: Option<String>,

    /// Template directory prefix filter (Plan 02 applies filtering).
    #[arg(long = "type")]
    pub type_: Option<String>,

    /// List resolved outputs without writing (Wave 1: no fs_writer calls).
    #[arg(long = "dry-run")]
    pub dry_run: bool,

    #[arg(long = "force", default_value_t = false)]
    pub force: bool,

    /// Output root directory (layer-3 fallback when no front-matter or templates.outputs entry).
    #[arg(long = "output")]
    pub output: Option<std::path::PathBuf>,

    /// Per-call variable override: `--var key=value` (repeatable). Keys must be
    /// declared in `.crud/templates/_variables.toml`.
    #[arg(long = "var", value_name = "KEY=VALUE")]
    pub var: Vec<String>,
}

/// `crud-cli validate` flags (parity with `gen --type`).
#[derive(Parser, Debug, Default)]
pub struct ValidateArgs {
    /// Template directory prefix filter (comma-separated).
    #[arg(long = "type")]
    pub type_: Option<String>,
}

impl GenArgs {
    /// Wave-1 validation: `--fields` and `--file` are mutually exclusive (D-G10).
    pub fn validate_inputs(&self) -> Result<(), ErrorEnvelope> {
        if self.fields.is_some() && self.file.is_some() {
            return Err(ErrorEnvelope::user_error_with_reason(
                "cannot use --fields and --file together",
                "fields_file_mutex",
                serde_json::Map::new(),
                "provide either --fields or --file, not both",
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, ValueEnum)]
#[value(rename_all = "kebab-case")]
pub enum SetupBackend {
    #[value(name = "spring-boot")]
    SpringBoot,
    Nest,
    None,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
#[value(rename_all = "kebab-case")]
pub enum SetupFrontend {
    Vue,
    React,
    None,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
#[value(rename_all = "kebab-case")]
pub enum SetupComponentLibrary {
    #[value(name = "element-plus")]
    ElementPlus,
    Antd,
    #[value(name = "naive-ui")]
    NaiveUi,
    None,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
#[value(rename_all = "kebab-case")]
pub enum SetupOverwritePolicy {
    Never,
    #[value(name = "force-only")]
    ForceOnly,
    Always,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
#[value(rename_all = "kebab-case")]
pub enum SetupEnabledTypes {
    All,
    Backend,
    Frontend,
}

impl From<SetupBackend> for Backend {
    fn from(v: SetupBackend) -> Self {
        match v {
            SetupBackend::SpringBoot => Backend::SpringBoot,
            SetupBackend::Nest => Backend::Nest,
            SetupBackend::None => Backend::None,
        }
    }
}

impl From<SetupFrontend> for Frontend {
    fn from(v: SetupFrontend) -> Self {
        match v {
            SetupFrontend::Vue => Frontend::Vue,
            SetupFrontend::React => Frontend::React,
            SetupFrontend::None => Frontend::None,
        }
    }
}

impl From<SetupComponentLibrary> for ComponentLibrary {
    fn from(v: SetupComponentLibrary) -> Self {
        match v {
            SetupComponentLibrary::ElementPlus => ComponentLibrary::ElementPlus,
            SetupComponentLibrary::Antd => ComponentLibrary::Antd,
            SetupComponentLibrary::NaiveUi => ComponentLibrary::NaiveUi,
            SetupComponentLibrary::None => ComponentLibrary::None,
        }
    }
}

impl From<SetupOverwritePolicy> for OverwritePolicy {
    fn from(v: SetupOverwritePolicy) -> Self {
        match v {
            SetupOverwritePolicy::Never => OverwritePolicy::Never,
            SetupOverwritePolicy::ForceOnly => OverwritePolicy::ForceOnly,
            SetupOverwritePolicy::Always => OverwritePolicy::Always,
        }
    }
}

impl From<SetupEnabledTypes> for EnabledTypes {
    fn from(v: SetupEnabledTypes) -> Self {
        match v {
            SetupEnabledTypes::All => EnabledTypes::All,
            SetupEnabledTypes::Backend => EnabledTypes::Backend,
            SetupEnabledTypes::Frontend => EnabledTypes::Frontend,
        }
    }
}

impl SetupArgs {
    /// True when the project scope has any dimension flag (skips wizard).
    #[must_use]
    pub fn is_project_non_interactive(&self) -> bool {
        self.backend.is_some()
            || self.frontend.is_some()
            || self.component_library.is_some()
    }

    /// True when the user scope has any dimension flag (skips wizard).
    #[must_use]
    pub fn is_user_non_interactive(&self) -> bool {
        self.user_name.is_some()
            || self.user_email.is_some()
            || self.overwrite_policy.is_some()
            || self.enabled_types.is_some()
    }

    /// True iff `--project` was passed.
    #[must_use]
    pub fn writes_project(&self) -> bool {
        self.project
    }

    /// Builds flag overlay for runtime merge pipeline.
    #[must_use]
    pub fn flag_overlay(&self) -> SetupFlagOverlay {
        SetupFlagOverlay {
            backend: self.backend.map(Into::into),
            frontend: self.frontend.map(Into::into),
            component_library: self.component_library.map(Into::into),
            overwrite_policy: self.overwrite_policy.map(Into::into),
            enabled_types: self.enabled_types.map(Into::into),
        }
    }

    /// Validates the three project dimensions are present (flag mode).
    pub fn require_project_non_interactive(&self) -> Result<SetupSelections, ErrorEnvelope> {
        let backend = self.backend.ok_or_else(|| missing_flag("backend"))?;
        let frontend = self.frontend.ok_or_else(|| missing_flag("frontend"))?;
        let component_library = self
            .component_library
            .ok_or_else(|| missing_flag("component-library"))?;
        Ok(SetupSelections {
            backend: backend.into(),
            frontend: frontend.into(),
            component_library: component_library.into(),
        })
    }

    /// Validates required user dimensions for flag mode.
    pub fn require_user_non_interactive(&self) -> Result<UserSelections, ErrorEnvelope> {
        let name = self
            .user_name
            .clone()
            .ok_or_else(|| missing_flag("user-name"))?;
        let email = self
            .user_email
            .clone()
            .ok_or_else(|| missing_flag("user-email"))?;
        let overwrite_policy = self
            .overwrite_policy
            .ok_or_else(|| missing_flag("overwrite-policy"))?;
        let enabled_types = self
            .enabled_types
            .map(Into::into)
            .unwrap_or(EnabledTypes::All);
        if name.trim().is_empty() {
            return Err(empty_flag("user-name"));
        }
        if email.trim().is_empty() {
            return Err(empty_flag("user-email"));
        }
        Ok(UserSelections {
            name,
            email,
            overwrite_policy: overwrite_policy.into(),
            enabled_types,
        })
    }

    /// Materializes project config from flags only (no file merge).
    pub fn to_setup_config(&self) -> Result<SetupConfig, ErrorEnvelope> {
        let selections = self.require_project_non_interactive()?;
        Ok(SetupConfig::from_selections(selections))
    }

    /// Materializes user config from flags only.
    pub fn to_user_config(&self) -> Result<SetupUserConfig, ErrorEnvelope> {
        let selections = self.require_user_non_interactive()?;
        Ok(SetupUserConfig::from_user_selections(selections))
    }
}

fn missing_flag(flag: &'static str) -> ErrorEnvelope {
    ErrorEnvelope::user_error(
        format!("missing required flag --{flag}"),
        Some(flag),
        None,
        format!("provide --{flag} with a value from the locked enum set"),
    )
}

fn empty_flag(flag: &'static str) -> ErrorEnvelope {
    ErrorEnvelope::user_error(
        format!("--{flag} must not be empty"),
        Some(flag),
        None,
        format!("provide a non-empty value for --{flag}"),
    )
}

/// Parses argv; maps clap failures to `UserError` envelope (D-09).
pub fn try_parse_cli<I, T>(args: I) -> Result<Cli, ErrorEnvelope>
where
    I: IntoIterator<Item = T>,
    T: Into<OsString> + Clone,
{
    Cli::try_parse_from(args).map_err(clap_to_user_error)
}

/// Maps clap errors; returns `None` for help/version (caller should exit 0).
pub fn try_parse_cli_or_help<I, T>(args: I) -> Result<Option<Cli>, ErrorEnvelope>
where
    I: IntoIterator<Item = T>,
    T: Into<OsString> + Clone,
{
    match Cli::try_parse_from(args) {
        Ok(cli) => Ok(Some(cli)),
        Err(err) if err.kind() == clap::error::ErrorKind::DisplayHelp => {
            let _ = err.print();
            Ok(None)
        }
        Err(err) if err.kind() == clap::error::ErrorKind::DisplayVersion => {
            let _ = err.print();
            Ok(None)
        }
        Err(err) => Err(clap_to_user_error(err)),
    }
}

fn clap_to_user_error(err: clap::Error) -> ErrorEnvelope {
    let (flag, value) = clap_flag_value(&err);
    ErrorEnvelope::user_error(
        err.to_string(),
        flag,
        value.as_deref(),
        "fix the flag value and retry",
    )
}

fn clap_flag_value(err: &clap::Error) -> (Option<&'static str>, Option<String>) {
    let s = err.to_string();
    for flag in [
        "backend",
        "frontend",
        "component-library",
        "overwrite-policy",
        "enabled-types",
        "user-name",
        "user-email",
        "fields",
        "file",
        "package",
        "table",
        "type",
    ] {
        if s.contains(flag) {
            let value = s
                .split_whitespace()
                .find(|w| !w.starts_with('-') && *w != flag)
                .map(str::to_string);
            return (Some(flag), value);
        }
    }
    (None, None)
}

/// Emits envelope and returns exit code.
pub fn exit_with_envelope(envelope: &ErrorEnvelope) -> i32 {
    emit_failure(envelope);
    envelope.exit_code
}
