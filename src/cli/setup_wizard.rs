//! Interactive `setup` wizards: project and user.

use std::io::IsTerminal;

use inquire::error::InquireError;
use inquire::{Confirm, Select, Text};

use crate::core::config::{
    is_valid_lang_id, Backend, EnabledTypes, Frontend, PathsSection, SetupConfig, SetupSelections,
    SetupUserConfig, TemplateRef, UserSelections,
};
use crate::core::default_paths::paths_for_selections;
use crate::core::error::ErrorEnvelope;
use crate::core::git_info;
use crate::core::global_config::{lang_env_override, GlobalConfig};
use crate::core::i18n::{self, keys, Lang};
use crate::core::paths::global_config_toml;
use crate::core::template_meta_global::{
    find_template, list_installed_templates, InstalledTemplate,
};
use crate::core::type_map::Fallback;

use super::agent_mode::is_agent_active;
use super::args::{SetupEnabledTypes, SetupOverwritePolicy};
use super::output::emit_success;

/// Ensures a UI language preference exists before running a wizard (first-run).
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
        Err(_) => return,
    };
    i18n::set(lang);
    cfg.set_lang(lang);
    let _ = cfg.save(&path);
}

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

/// Runs the project wizard and returns the canonical project config.
pub fn run_project_wizard() -> Result<SetupConfig, ErrorEnvelope> {
    let selections = collect_project_selections()?;
    let mut cfg = SetupConfig::from_selections(selections);
    if let Some(tref) = &cfg.project.template.clone() {
        if let Ok(installed) = find_template(&tref.name, tref.version.as_deref()) {
            if let Some(paths) = installed.manifest.paths {
                cfg.paths = paths;
            }
        }
    }
    cfg.paths = prompt_paths(cfg.paths)?;
    cfg.type_map.fallback = prompt_type_map_fallback()?;
    Ok(cfg)
}

/// Runs the user wizard and returns the canonical user config.
pub fn run_user_wizard() -> Result<SetupUserConfig, ErrorEnvelope> {
    let selections = collect_user_selections()?;
    Ok(SetupUserConfig::from_user_selections(selections))
}

/// Test-friendly mapping of user wizard answers (kept for legacy test code).
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
    let installed = list_installed_templates();
    if let Some(picked) = prompt_template_choice(&installed)? {
        let backend = picked.manifest.backend.clone();
        let frontend = picked.manifest.frontend.clone();
        let template = Some(TemplateRef {
            name: picked.name.clone(),
            version: Some(picked.version.clone()),
        });
        return Ok(SetupSelections {
            backend,
            frontend,
            template,
        });
    }
    let backend = prompt_backend()?;
    let frontend = prompt_frontend()?;
    Ok(SetupSelections {
        backend,
        frontend,
        template: None,
    })
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

/// Presents installed templates plus a "manual" option. Returns `None` when
/// the user picks "manual" or no templates are installed.
fn prompt_template_choice(
    installed: &[InstalledTemplate],
) -> Result<Option<InstalledTemplate>, ErrorEnvelope> {
    if installed.is_empty() {
        emit_success(Some(i18n::t(keys::WIZARD_TEMPLATE_NO_TEMPLATES)));
        return Ok(None);
    }
    let manual_label = i18n::t(keys::WIZARD_TEMPLATE_MANUAL_OPTION).to_string();
    let mut labels: Vec<String> = installed
        .iter()
        .map(|t| {
            format!(
                "{name}@{ver}  ({backend} + {frontend})",
                name = t.name,
                ver = t.version,
                backend = t.manifest.backend.as_key(),
                frontend = t.manifest.frontend.as_key()
            )
        })
        .collect();
    labels.push(manual_label.clone());

    let header = i18n::t(keys::WIZARD_TEMPLATE_DETECTED_HEADER);
    emit_success(Some(header));
    let choice = Select::new("template", labels.clone())
        .prompt()
        .map_err(inquire_to_user_error)?;
    if choice == manual_label {
        return Ok(None);
    }
    let idx = labels
        .iter()
        .position(|l| *l == choice)
        .ok_or_else(invalid_selection)?;
    Ok(installed.get(idx).cloned())
}

fn prompt_backend() -> Result<Backend, ErrorEnvelope> {
    let custom = i18n::t(keys::WIZARD_TEMPLATE_CUSTOM_INPUT).to_string();
    let options = [
        ("java", Backend::Java),
        ("typescript", Backend::TypeScript),
        ("go", Backend::Go),
        ("python", Backend::Python),
        ("none", Backend::None),
    ];
    let mut labels: Vec<String> = options.iter().map(|(l, _)| (*l).to_string()).collect();
    labels.push(custom.clone());
    let choice = Select::new(
        i18n::t(keys::WIZARD_TEMPLATE_CHOOSE_BACKEND),
        labels.clone(),
    )
    .with_help_message(i18n::t(keys::WIZARD_HELP_BACKEND))
    .prompt()
    .map_err(inquire_to_user_error)?;
    if choice == custom {
        let raw = Text::new("backend")
            .with_validator(non_empty_validator())
            .prompt()
            .map_err(inquire_to_user_error)?;
        return Backend::parse(raw.trim()).map_err(|_| invalid_lang_name(raw.trim()));
    }
    let idx = labels
        .iter()
        .position(|l| *l == choice)
        .ok_or_else(invalid_selection)?;
    Ok(options[idx].1.clone())
}

fn prompt_frontend() -> Result<Frontend, ErrorEnvelope> {
    let custom = i18n::t(keys::WIZARD_TEMPLATE_CUSTOM_INPUT).to_string();
    let options = [
        ("vue", Frontend::Vue),
        ("react", Frontend::React),
        ("none", Frontend::None),
    ];
    let mut labels: Vec<String> = options.iter().map(|(l, _)| (*l).to_string()).collect();
    labels.push(custom.clone());
    let choice = Select::new(
        i18n::t(keys::WIZARD_TEMPLATE_CHOOSE_FRONTEND),
        labels.clone(),
    )
    .prompt()
    .map_err(inquire_to_user_error)?;
    if choice == custom {
        let raw = Text::new("frontend")
            .with_validator(non_empty_validator())
            .prompt()
            .map_err(inquire_to_user_error)?;
        return Frontend::parse(raw.trim()).map_err(|_| invalid_lang_name(raw.trim()));
    }
    let idx = labels
        .iter()
        .position(|l| *l == choice)
        .ok_or_else(invalid_selection)?;
    Ok(options[idx].1.clone())
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
    Ok(options[label_index(choice, &labels)?])
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
    Ok(options[label_index(choice, &labels)?])
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

fn invalid_selection() -> ErrorEnvelope {
    ErrorEnvelope::user_error(
        i18n::t(keys::WIZARD_INVALID_SELECTION_MSG),
        None,
        None,
        i18n::t(keys::WIZARD_INVALID_SELECTION_HINT),
    )
}

fn invalid_lang_name(value: &str) -> ErrorEnvelope {
    let mut details = serde_json::Map::new();
    details.insert("value".into(), serde_json::Value::String(value.to_string()));
    ErrorEnvelope::user_error_with_reason(
        format!("invalid language identifier: {value:?}"),
        "invalid_lang_id",
        details,
        i18n::t(keys::WIZARD_TEMPLATE_INVALID_LANG_NAME),
    )
}

fn inquire_to_user_error(err: InquireError) -> ErrorEnvelope {
    let (msg, value) = match &err {
        InquireError::OperationCanceled | InquireError::OperationInterrupted => (
            i18n::t(keys::WIZARD_CANCELLED_MSG).to_string(),
            None::<String>,
        ),
        other => (other.to_string(), None::<String>),
    };
    ErrorEnvelope::user_error(
        msg,
        None,
        value.as_deref(),
        i18n::t(keys::WIZARD_CANCELLED_HINT),
    )
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
    let lang_keys: Vec<String> = paths.lang.keys().cloned().collect();
    for key in lang_keys {
        let label = format!("paths.lang.{key}");
        let current = paths.lang.get(&key).cloned();
        if let Some(new_value) = prompt_optional_path(&label, current)? {
            paths.lang.insert(key, new_value);
        } else {
            paths.lang.remove(&key);
        }
    }
    let aux_keys: Vec<String> = paths.aux.keys().cloned().collect();
    for key in aux_keys {
        let label = format!("paths.aux.{key}");
        let current = paths.aux.get(&key).cloned();
        if let Some(new_value) = prompt_optional_path(&label, current)? {
            paths.aux.insert(key, new_value);
        } else {
            paths.aux.remove(&key);
        }
    }
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
    let mut parts: Vec<String> = Vec::new();
    for (k, v) in &p.lang {
        parts.push(format!("lang.{k}={v}"));
    }
    for (k, v) in &p.aux {
        parts.push(format!("aux.{k}={v}"));
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

// Touch otherwise unused imports
#[allow(dead_code)]
fn _imports_assert(_t: EnabledTypes, _v: bool, _p: PathsSection) {
    let _ = is_valid_lang_id("x");
    let _ = paths_for_selections(&Backend::None, &Frontend::None);
}
