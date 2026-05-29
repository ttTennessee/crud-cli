//! Interactive `setup` wizards: project and user (CONF-01).

use std::io::IsTerminal;

use inquire::error::InquireError;
use inquire::{Confirm, Select, Text};

use crate::core::config::{
    EnabledTypes, PathsSection, SetupConfig, SetupSelections, SetupUserConfig, UserSelections,
};
use crate::core::error::ErrorEnvelope;
use crate::core::git_info;
use crate::core::global_config::{lang_env_override, GlobalConfig};
use crate::core::i18n::{self, keys, Lang};
use crate::core::paths::global_config_toml;
use crate::core::type_map::Fallback;

use super::agent_mode::is_agent_active;
use super::args::{
    SetupBackend, SetupComponentLibrary, SetupEnabledTypes, SetupFrontend, SetupOverwritePolicy,
};

/// Ensures a UI language preference exists before running a wizard (first-run).
///
/// Resolution: agent / non-interactive → no prompt (locale already resolved by
/// [`crate::cli::init_locale`]); `CRUD_LANG` set → honor it without persisting;
/// stored preference → apply it; otherwise prompt the user once and persist the
/// choice to `~/.crud/config.toml`.
pub fn ensure_language_preference() {
    if is_agent_active() || !std::io::stdin().is_terminal() {
        return;
    }
    if let Some(lang) = lang_env_override() {
        i18n::set(lang);
        return;
    }
    let path = match global_config_toml() {
        Ok(p) => p,
        Err(_) => return,
    };
    let mut cfg = GlobalConfig::load_or_default(&path);
    if let Some(lang) = cfg.lang() {
        i18n::set(lang);
        return;
    }
    let lang = match prompt_language() {
        Ok(l) => l,
        // Cancelled selection: keep the default locale, do not persist.
        Err(_) => return,
    };
    i18n::set(lang);
    cfg.set_lang(lang);
    // Best-effort persistence; a write failure must not block setup.
    let _ = cfg.save(&path);
}

/// Prompts for a UI language. Labels stay language-neutral on purpose.
pub fn prompt_language() -> Result<Lang, ErrorEnvelope> {
    const EN_LABEL: &str = "English";
    const ZH_LABEL: &str = "中文";
    let choice = Select::new("Select language / 选择语言", vec![EN_LABEL, ZH_LABEL])
        .prompt()
        .map_err(inquire_to_user_error)?;
    Ok(if choice == ZH_LABEL {
        Lang::Zh
    } else {
        Lang::En
    })
}

/// Runs the project wizard and returns the canonical project config .
pub fn run_project_wizard() -> Result<SetupConfig, ErrorEnvelope> {
    let selections = collect_project_selections()?;
    let mut cfg = SetupConfig::from_selections(selections);
    cfg.paths = prompt_paths(cfg.paths)?;
    cfg.type_map.fallback = prompt_type_map_fallback()?;
    Ok(cfg)
}

/// Runs the user wizard and returns the canonical user config.
pub fn run_user_wizard() -> Result<SetupUserConfig, ErrorEnvelope> {
    let selections = collect_user_selections()?;
    Ok(SetupUserConfig::from_user_selections(selections))
}

/// Test-friendly mapping of project wizard answers.
#[must_use]
pub fn selections_from_answers(
    backend: SetupBackend,
    frontend: SetupFrontend,
    component_library: SetupComponentLibrary,
) -> SetupSelections {
    SetupSelections {
        backend: backend.into(),
        frontend: frontend.into(),
        component_library: component_library.into(),
    }
}

/// Test-friendly mapping of user wizard answers.
#[must_use]
pub fn user_selections_from_answers(
    name: String,
    email: String,
    overwrite_policy: SetupOverwritePolicy,
    enabled_types: SetupEnabledTypes,
) -> UserSelections {
    UserSelections {
        name,
        email,
        overwrite_policy: overwrite_policy.into(),
        enabled_types: enabled_types.into(),
    }
}

fn collect_project_selections() -> Result<SetupSelections, ErrorEnvelope> {
    let backend = prompt_backend()?;
    let frontend = prompt_frontend()?;
    let component_library = prompt_component_library()?;
    Ok(selections_from_answers(backend, frontend, component_library))
}

fn collect_user_selections() -> Result<UserSelections, ErrorEnvelope> {
    let git = git_info::read();
    let name = prompt_name(&git.user_name)?;
    let email = prompt_email(&git.user_email)?;
    let overwrite_policy = prompt_overwrite_policy()?;
    let enabled_types = prompt_enabled_types()?;
    Ok(user_selections_from_answers(
        name,
        email,
        overwrite_policy,
        enabled_types,
    ))
}

fn prompt_backend() -> Result<SetupBackend, ErrorEnvelope> {
    let options = [
        SetupBackend::SpringBoot,
        SetupBackend::Nest,
        SetupBackend::None,
    ];
    let labels: Vec<&str> = options.iter().map(|o| backend_label(*o)).collect();
    let choice = Select::new("backend", labels.clone())
        .with_help_message(i18n::t(keys::WIZARD_HELP_BACKEND))
        .prompt()
        .map_err(inquire_to_user_error)?;
    Ok(options[label_index(&choice, &labels)?])
}

fn prompt_frontend() -> Result<SetupFrontend, ErrorEnvelope> {
    let options = [SetupFrontend::Vue, SetupFrontend::React, SetupFrontend::None];
    let labels: Vec<&str> = options.iter().map(|o| frontend_label(*o)).collect();
    let choice = Select::new("frontend", labels.clone())
        .prompt()
        .map_err(inquire_to_user_error)?;
    Ok(options[label_index(&choice, &labels)?])
}

fn prompt_component_library() -> Result<SetupComponentLibrary, ErrorEnvelope> {
    let options = [
        SetupComponentLibrary::ElementPlus,
        SetupComponentLibrary::Antd,
        SetupComponentLibrary::NaiveUi,
        SetupComponentLibrary::None,
    ];
    let labels: Vec<&str> = options.iter().map(|o| component_library_label(*o)).collect();
    let choice = Select::new("component-library", labels.clone())
        .prompt()
        .map_err(inquire_to_user_error)?;
    Ok(options[label_index(&choice, &labels)?])
}

fn prompt_overwrite_policy() -> Result<SetupOverwritePolicy, ErrorEnvelope> {
    let options = [
        SetupOverwritePolicy::Never,
        SetupOverwritePolicy::ForceOnly,
        SetupOverwritePolicy::Always,
    ];
    let labels: Vec<&str> = options.iter().map(|o| overwrite_policy_label(*o)).collect();
    let choice = Select::new("overwrite-policy", labels.clone())
        .prompt()
        .map_err(inquire_to_user_error)?;
    Ok(options[label_index(&choice, &labels)?])
}

fn prompt_enabled_types() -> Result<SetupEnabledTypes, ErrorEnvelope> {
    let options = [
        SetupEnabledTypes::All,
        SetupEnabledTypes::Backend,
        SetupEnabledTypes::Frontend,
    ];
    let labels: Vec<&str> = options.iter().map(|o| enabled_types_label(*o)).collect();
    let choice = Select::new("enabled-types", labels.clone())
        .with_help_message(i18n::t(keys::WIZARD_HELP_ENABLED_TYPES))
        .prompt()
        .map_err(inquire_to_user_error)?;
    Ok(options[label_index(&choice, &labels)?])
}

fn prompt_name(default: &str) -> Result<String, ErrorEnvelope> {
    let mut text = Text::new("name").with_help_message(i18n::t(keys::WIZARD_HELP_NAME));
    if !default.is_empty() {
        text = text.with_default(default);
    }
    let value = text
        .with_validator(non_empty_validator())
        .prompt()
        .map_err(inquire_to_user_error)?;
    Ok(value.trim().to_string())
}

fn prompt_email(default: &str) -> Result<String, ErrorEnvelope> {
    let mut text = Text::new("email").with_help_message(i18n::t(keys::WIZARD_HELP_EMAIL));
    if !default.is_empty() {
        text = text.with_default(default);
    }
    let value = text
        .with_validator(non_empty_validator())
        .prompt()
        .map_err(inquire_to_user_error)?;
    Ok(value.trim().to_string())
}

fn non_empty_validator() -> inquire::validator::ValueRequiredValidator {
    inquire::validator::ValueRequiredValidator::new(i18n::t(keys::WIZARD_NOT_EMPTY))
}

fn label_index(choice: &str, labels: &[&str]) -> Result<usize, ErrorEnvelope> {
    labels.iter().position(|l| *l == choice).ok_or_else(|| {
        ErrorEnvelope::user_error(
            i18n::t(keys::WIZARD_INVALID_SELECTION_MSG),
            None,
            Some(choice),
            i18n::t(keys::WIZARD_INVALID_SELECTION_HINT),
        )
    })
}

fn inquire_to_user_error(err: InquireError) -> ErrorEnvelope {
    let (msg, value) = match &err {
        InquireError::OperationCanceled | InquireError::OperationInterrupted => {
            (i18n::t(keys::WIZARD_CANCELLED_MSG).to_string(), None::<String>)
        }
        other => (other.to_string(), None::<String>),
    };
    ErrorEnvelope::user_error(
        msg,
        None,
        value.as_deref(),
        i18n::t(keys::WIZARD_CANCELLED_HINT),
    )
}

fn backend_label(b: SetupBackend) -> &'static str {
    match b {
        SetupBackend::SpringBoot => "spring-boot",
        SetupBackend::Nest => "nest",
        SetupBackend::None => "none",
    }
}

fn frontend_label(f: SetupFrontend) -> &'static str {
    match f {
        SetupFrontend::Vue => "vue",
        SetupFrontend::React => "react",
        SetupFrontend::None => "none",
    }
}

fn component_library_label(c: SetupComponentLibrary) -> &'static str {
    match c {
        SetupComponentLibrary::ElementPlus => "element-plus",
        SetupComponentLibrary::Antd => "antd",
        SetupComponentLibrary::NaiveUi => "naive-ui",
        SetupComponentLibrary::None => "none",
    }
}

fn overwrite_policy_label(p: SetupOverwritePolicy) -> &'static str {
    match p {
        SetupOverwritePolicy::Never => "never",
        SetupOverwritePolicy::ForceOnly => "force-only",
        SetupOverwritePolicy::Always => "always",
    }
}

fn enabled_types_label(t: SetupEnabledTypes) -> &'static str {
    match t {
        SetupEnabledTypes::All => "all",
        SetupEnabledTypes::Backend => "backend",
        SetupEnabledTypes::Frontend => "frontend",
    }
}

fn prompt_paths(defaults: PathsSection) -> Result<PathsSection, ErrorEnvelope> {
    let summary = summarize_paths(&defaults);
    let help = if summary.is_empty() {
        i18n::t(keys::WIZARD_PATHS_NONE).to_string()
    } else {
        i18n::tf(keys::WIZARD_PATHS_DEFAULTS, &[("summary", &summary)])
    };
    let customize = Confirm::new(i18n::t(keys::WIZARD_PATHS_CUSTOMIZE))
        .with_default(false)
        .with_help_message(&help)
        .prompt()
        .map_err(inquire_to_user_error)?;
    if !customize {
        return Ok(defaults);
    }
    let mut paths = defaults;
    paths.java_base = prompt_optional_path("paths.java_base", paths.java_base)?;
    paths.resources_base = prompt_optional_path("paths.resources_base", paths.resources_base)?;
    paths.doc_base = prompt_optional_path("paths.doc_base", paths.doc_base)?;
    paths.nest_base = prompt_optional_path("paths.nest_base", paths.nest_base)?;
    paths.vue_base = prompt_optional_path("paths.vue_base", paths.vue_base)?;
    paths.react_base = prompt_optional_path("paths.react_base", paths.react_base)?;
    Ok(paths)
}

fn prompt_optional_path(
    label: &str,
    default: Option<String>,
) -> Result<Option<String>, ErrorEnvelope> {
    let Some(current) = default else {
        return Ok(None);
    };
    let value = Text::new(label)
        .with_default(&current)
        .prompt()
        .map_err(inquire_to_user_error)?;
    let trimmed = value.trim();
    if trimmed.is_empty() {
        Ok(None)
    } else {
        Ok(Some(trimmed.to_string()))
    }
}

fn summarize_paths(p: &PathsSection) -> String {
    let mut parts = Vec::new();
    for (k, v) in [
        ("java_base", &p.java_base),
        ("resources_base", &p.resources_base),
        ("doc_base", &p.doc_base),
        ("nest_base", &p.nest_base),
        ("vue_base", &p.vue_base),
        ("react_base", &p.react_base),
    ] {
        if let Some(value) = v {
            parts.push(format!("{k}={value}"));
        }
    }
    parts.join(", ")
}

fn prompt_type_map_fallback() -> Result<Fallback, ErrorEnvelope> {
    let passthrough = i18n::t(keys::WIZARD_TYPEMAP_PASSTHROUGH);
    let error = i18n::t(keys::WIZARD_TYPEMAP_ERROR);
    let literal = i18n::t(keys::WIZARD_TYPEMAP_LITERAL);
    let labels = vec![passthrough, error, literal];
    let choice = Select::new("type_map.fallback", labels)
        .with_help_message(i18n::t(keys::WIZARD_TYPEMAP_HELP))
        .prompt()
        .map_err(inquire_to_user_error)?;
    Ok(if choice == passthrough {
        Fallback::Passthrough
    } else if choice == error {
        Fallback::Error
    } else if choice == literal {
        let value = Text::new("type_map.fallback literal")
            .with_default("any")
            .with_help_message(i18n::t(keys::WIZARD_TYPEMAP_LITERAL_HELP))
            .with_validator(non_empty_validator())
            .prompt()
            .map_err(inquire_to_user_error)?;
        Fallback::Literal(value.trim().to_string())
    } else {
        return Err(ErrorEnvelope::user_error(
            i18n::t(keys::WIZARD_INVALID_SELECTION_MSG),
            None,
            Some(choice),
            i18n::t(keys::WIZARD_INVALID_SELECTION_HINT),
        ));
    })
}

// Silence unused-import diagnostics when `EnabledTypes`/`git_info` are only
// referenced from wizard runtime paths.
#[allow(dead_code)]
fn _enabled_types_assert(_t: EnabledTypes) {}
