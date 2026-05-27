//! clap surface for `setup` (D-08, CONF-02, CONF-08, FOUND-04).

use clap::{Parser, Subcommand, ValueEnum};
use std::ffi::OsString;

use crate::core::config::{
    Backend, ComponentLibrary, Frontend, OverwritePolicy, SetupConfig, SetupFlagOverlay,
    SetupSelections,
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
    /// Create or refresh project setup configuration.
    Setup(SetupArgs),
    /// Generate CRUD files from project templates.
    Gen(GenArgs),
    /// Validate project templates before generation.
    Validate(ValidateArgs),
}

/// `crud-cli setup` flags (D-08).
#[derive(Parser, Debug, Default)]
pub struct SetupArgs {
    #[arg(long = "backend", value_enum)]
    pub backend: Option<SetupBackend>,

    #[arg(long = "frontend", value_enum)]
    pub frontend: Option<SetupFrontend>,

    #[arg(long = "component-library", value_enum)]
    pub component_library: Option<SetupComponentLibrary>,

    #[arg(long = "overwrite-policy", value_enum)]
    pub overwrite_policy: Option<SetupOverwritePolicy>,

    /// Allow writes when `overwrite-policy=force-only` and target exists (CONF-08).
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

impl SetupArgs {
    /// True when any setup dimension flag was passed (non-interactive path).
    #[must_use]
    pub fn is_non_interactive(&self) -> bool {
        self.backend.is_some()
            || self.frontend.is_some()
            || self.component_library.is_some()
            || self.overwrite_policy.is_some()
    }

    /// Builds flag overlay for merge pipeline.
    #[must_use]
    pub fn flag_overlay(&self) -> SetupFlagOverlay {
        SetupFlagOverlay {
            backend: self.backend.map(|v| v.into()),
            frontend: self.frontend.map(|v| v.into()),
            component_library: self.component_library.map(|v| v.into()),
            overwrite_policy: self.overwrite_policy.map(|v| v.into()),
        }
    }

    /// Validates all four dimensions are present for flag mode (D-09).
    pub fn require_non_interactive_fields(&self) -> Result<SetupSelections, ErrorEnvelope> {
        let backend = self.backend.ok_or_else(|| missing_flag("backend"))?;
        let frontend = self.frontend.ok_or_else(|| missing_flag("frontend"))?;
        let component_library = self
            .component_library
            .ok_or_else(|| missing_flag("component-library"))?;
        let overwrite_policy = self
            .overwrite_policy
            .ok_or_else(|| missing_flag("overwrite-policy"))?;
        Ok(SetupSelections {
            backend: backend.into(),
            frontend: frontend.into(),
            component_library: component_library.into(),
            overwrite_policy: overwrite_policy.into(),
        })
    }

    /// Materializes config from flags only (no file merge).
    pub fn to_setup_config(&self) -> Result<SetupConfig, ErrorEnvelope> {
        let selections = self.require_non_interactive_fields()?;
        Ok(SetupConfig::from_selections(selections))
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
