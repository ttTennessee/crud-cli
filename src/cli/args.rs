//! clap surface for `setup` / `gen` / `validate` / `template`.

use clap::{Parser, Subcommand, ValueEnum};
use std::ffi::OsString;

use crate::core::config::{
    Backend, EnabledTypes, Frontend, OverwritePolicy, SetupConfig, SetupFlagOverlay,
    SetupSelections, SetupUserConfig, TemplateRef, UserSelections,
};
use crate::core::error::ErrorEnvelope;
use crate::core::i18n::{self, keys};
use crate::core::type_map::Fallback;

use super::output::emit_failure;

/// Root CLI parser.
#[derive(Parser, Debug)]
#[command(name = "crud-cli", version, about)]
pub struct Cli {
    /// Agent/machine mode: JSON errors on stderr, empty success stdout.
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
    /// Manage installed templates under `~/.crud/templates/`.
    Template(TemplateArgs),
    /// Start the Model Context Protocol server on stdio (for Cursor / MCP clients).
    #[cfg(feature = "mcp")]
    Mcp(McpArgs),
}

/// `crud-cli mcp` flags.
#[derive(Parser, Debug, Default)]
#[cfg(feature = "mcp")]
pub struct McpArgs {
    /// Project root (overrides MCP workspace roots and process cwd).
    #[arg(short = 'p', long = "path", value_name = "DIR")]
    pub path: Option<String>,
}

/// `crud-cli setup` flags. Default scope = user; `--project` switches target.
#[derive(Parser, Debug, Default)]
pub struct SetupArgs {
    /// Write the shared project config (`.crud/setup.toml`) instead of the
    /// per-developer user config (`.crud/setup.user.toml`).
    #[arg(long = "project", default_value_t = false)]
    pub project: bool,

    /// Project backend language. Known: java, typescript, go, python, none.
    /// Any other lowercase identifier becomes `Backend::Custom`.
    #[arg(long = "backend", value_name = "LANG")]
    pub backend: Option<String>,

    /// Project frontend. Known: vue, react, none. Other lowercase ids become custom.
    #[arg(long = "frontend", value_name = "LANG")]
    pub frontend: Option<String>,

    /// Use an installed template by `name[@version]`. Recorded under
    /// `[project].template`.
    #[arg(long = "template", value_name = "NAME[@VERSION]")]
    pub template: Option<String>,

    /// Repeatable `--lang KEY=PATH` for `[paths.lang]` (e.g. `--lang java=src/main/java`).
    #[arg(long = "lang", value_name = "KEY=PATH")]
    pub lang: Vec<String>,

    /// Repeatable `--aux KEY=PATH` for `[paths.aux]` (e.g. `--aux doc=docs`).
    #[arg(long = "aux", value_name = "KEY=PATH")]
    pub aux: Vec<String>,

    // User-scope flags
    #[arg(long = "overwrite-policy", value_enum)]
    pub overwrite_policy: Option<SetupOverwritePolicy>,

    #[arg(long = "enabled-types", value_enum)]
    pub enabled_types: Option<SetupEnabledTypes>,

    #[arg(long = "user-name")]
    pub user_name: Option<String>,

    #[arg(long = "user-email")]
    pub user_email: Option<String>,

    /// Unknown-type fallback policy for [type_map]. Accepts
    /// `passthrough`, `error`, or any other string (treated as a literal
    /// replacement value).
    #[arg(long = "type-map-fallback", value_name = "POLICY")]
    pub type_map_fallback: Option<String>,

    /// Overwrite the target file without the interactive confirm.
    #[arg(long = "force", default_value_t = false)]
    pub force: bool,
}

/// `crud-cli gen` flags .
#[derive(Parser, Debug, Default)]
pub struct GenArgs {
    pub name: Option<String>,

    #[arg(long = "fields")]
    pub fields: Option<String>,

    #[arg(long = "file")]
    pub file: Option<std::path::PathBuf>,

    #[arg(long = "package")]
    pub package: Option<String>,

    #[arg(long = "table")]
    pub table: Option<String>,

    /// Business description of the table/entity (`{{table_comment}}` in templates).
    #[arg(long = "table-comment")]
    pub table_comment: Option<String>,

    #[arg(long = "type")]
    pub type_: Option<String>,

    #[arg(long = "dry-run")]
    pub dry_run: bool,

    /// Render to stdout instead of writing files (preview, e.g. confirm DDL
    /// before generating). Combine with `--type` to scope to one template set.
    #[arg(long = "stdout")]
    pub stdout: bool,

    #[arg(long = "force", default_value_t = false)]
    pub force: bool,

    #[arg(long = "output")]
    pub output: Option<std::path::PathBuf>,

    #[arg(long = "var", value_name = "KEY=VALUE")]
    pub var: Vec<String>,
}

/// `crud-cli validate` flags.
#[derive(Parser, Debug, Default)]
pub struct ValidateArgs {
    #[arg(long = "type")]
    pub type_: Option<String>,
}

/// `crud-cli template ...` — manage installed templates.
#[derive(Parser, Debug)]
pub struct TemplateArgs {
    #[command(subcommand)]
    pub command: TemplateCommand,
}

#[derive(Subcommand, Debug)]
pub enum TemplateCommand {
    /// Switch the project to an installed template.
    Use {
        /// `name` or `name@version`.
        name: String,
        /// Accept the switch without confirmation.
        #[arg(long = "yes", short = 'y', default_value_t = false)]
        yes: bool,
    },
    /// List installed templates under `~/.crud/templates/`.
    List,
    /// Download a template from a GitHub repo into `~/.crud/templates/`.
    ///
    /// Omit `<NAME>` to pick interactively; pass `<NAME>` alone to pick the
    /// version interactively; pass `<NAME>@<VERSION>` to install directly.
    Install {
        /// Optional `name` or `name@version`. Omitting it triggers the
        /// interactive picker (requires a TTY; not allowed in --agent mode).
        name: Option<String>,
        /// Override repo (`owner/repo[@ref]` or full GitHub URL). Defaults to
        /// `[templates].repo` in `~/.crud/config.toml`, then `ttTennessee/crud-templates`.
        #[arg(long = "repo", value_name = "OWNER/REPO[@REF]")]
        repo: Option<String>,
        /// Overwrite an already-installed `<name>/<version>/` directory.
        #[arg(long = "force", default_value_t = false)]
        force: bool,
    },
}

impl GenArgs {
    pub fn validate_inputs(&self) -> Result<(), ErrorEnvelope> {
        if self.fields.is_some() && self.file.is_some() {
            return Err(ErrorEnvelope::user_error_with_reason(
                "cannot use --fields and --file together",
                "fields_file_mutex",
                serde_json::Map::new(),
                i18n::t(keys::ERROR_CLI_FIELDS_FILE_MUTEX),
            ));
        }
        Ok(())
    }
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
            || self.template.is_some()
            || !self.lang.is_empty()
            || !self.aux.is_empty()
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

    fn parse_backend(&self) -> Result<Option<Backend>, ErrorEnvelope> {
        self.backend
            .as_deref()
            .map(|v| Backend::parse(v).map_err(|e| invalid_value("backend", v, e)))
            .transpose()
    }

    fn parse_frontend(&self) -> Result<Option<Frontend>, ErrorEnvelope> {
        self.frontend
            .as_deref()
            .map(|v| Frontend::parse(v).map_err(|e| invalid_value("frontend", v, e)))
            .transpose()
    }

    fn parse_template(&self) -> Result<Option<TemplateRef>, ErrorEnvelope> {
        self.template
            .as_deref()
            .map(|v| TemplateRef::parse(v).map_err(|e| invalid_value("template", v, e)))
            .transpose()
    }

    fn parse_path_kv(
        flag: &'static str,
        items: &[String],
    ) -> Result<std::collections::BTreeMap<String, String>, ErrorEnvelope> {
        let mut out = std::collections::BTreeMap::new();
        for raw in items {
            let (k, v) = raw.split_once('=').ok_or_else(|| {
                invalid_value(flag, raw, "expected KEY=PATH")
            })?;
            if k.trim().is_empty() || v.trim().is_empty() {
                return Err(invalid_value(flag, raw, "KEY and PATH must be non-empty"));
            }
            out.insert(k.trim().to_string(), v.trim().to_string());
        }
        Ok(out)
    }

    /// Builds flag overlay for runtime merge pipeline.
    pub fn flag_overlay(&self) -> Result<SetupFlagOverlay, ErrorEnvelope> {
        Ok(SetupFlagOverlay {
            backend: self.parse_backend()?,
            frontend: self.parse_frontend()?,
            template: self.parse_template()?,
            overwrite_policy: self.overwrite_policy.map(Into::into),
            enabled_types: self.enabled_types.map(Into::into),
            type_map_fallback: self
                .type_map_fallback
                .as_deref()
                .map(crate::core::config::parse_type_map_fallback),
            paths_lang: Self::parse_path_kv("lang", &self.lang)?,
            paths_aux: Self::parse_path_kv("aux", &self.aux)?,
        })
    }

    /// Returns the parsed fallback if `--type-map-fallback` was supplied.
    #[must_use]
    pub fn parsed_type_map_fallback(&self) -> Option<Fallback> {
        self.type_map_fallback
            .as_deref()
            .map(crate::core::config::parse_type_map_fallback)
    }

    /// Validates project dimensions are present (flag mode). Backend and
    /// frontend are required; template is optional.
    pub fn require_project_non_interactive(&self) -> Result<SetupSelections, ErrorEnvelope> {
        let backend = self.parse_backend()?.ok_or_else(|| missing_flag("backend"))?;
        let frontend = self.parse_frontend()?.ok_or_else(|| missing_flag("frontend"))?;
        let template = self.parse_template()?;
        Ok(SetupSelections {
            backend,
            frontend,
            template,
        })
    }

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
        let mut cfg = SetupConfig::from_selections(selections);
        for (k, v) in Self::parse_path_kv("lang", &self.lang)? {
            cfg.paths.lang.insert(k, v);
        }
        for (k, v) in Self::parse_path_kv("aux", &self.aux)? {
            cfg.paths.aux.insert(k, v);
        }
        if let Some(fb) = self.parsed_type_map_fallback() {
            cfg.type_map.fallback = fb;
        }
        Ok(cfg)
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
        i18n::tf(keys::ERROR_CLI_MISSING_FLAG, &[("flag", flag)]),
    )
}

fn empty_flag(flag: &'static str) -> ErrorEnvelope {
    ErrorEnvelope::user_error(
        format!("--{flag} must not be empty"),
        Some(flag),
        None,
        i18n::tf(keys::ERROR_CLI_EMPTY_FLAG, &[("flag", flag)]),
    )
}

fn invalid_value(flag: &'static str, value: &str, reason: &str) -> ErrorEnvelope {
    ErrorEnvelope::user_error(
        format!("--{flag} {value:?}: {reason}"),
        Some(flag),
        Some(value),
        i18n::t(keys::ERROR_CLI_CLAP_RETRY),
    )
}

/// Parses argv; maps clap failures to `UserError` envelope.
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
        i18n::t(keys::ERROR_CLI_CLAP_RETRY),
    )
}

fn clap_flag_value(err: &clap::Error) -> (Option<&'static str>, Option<String>) {
    let s = err.to_string();
    for flag in [
        "backend",
        "frontend",
        "template",
        "lang",
        "aux",
        "overwrite-policy",
        "enabled-types",
        "user-name",
        "user-email",
        "type-map-fallback",
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
