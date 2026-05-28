//! Interactive `setup` wizards: project and user (CONF-01).

use inquire::error::InquireError;
use inquire::{Select, Text};

use crate::core::config::{
    EnabledTypes, SetupConfig, SetupSelections, SetupUserConfig, UserSelections,
};
use crate::core::error::ErrorEnvelope;
use crate::core::git_info;

use super::args::{
    SetupBackend, SetupComponentLibrary, SetupEnabledTypes, SetupFrontend, SetupOverwritePolicy,
};

/// Runs the project wizard and returns the canonical project config (D-10).
pub fn run_project_wizard() -> Result<SetupConfig, ErrorEnvelope> {
    let selections = collect_project_selections()?;
    Ok(SetupConfig::from_selections(selections))
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
        .with_help_message("Project backend stack (D-08)")
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
        .with_help_message("Implicit --type filter for gen/validate")
        .prompt()
        .map_err(inquire_to_user_error)?;
    Ok(options[label_index(&choice, &labels)?])
}

fn prompt_name(default: &str) -> Result<String, ErrorEnvelope> {
    let mut text = Text::new("name").with_help_message("Used in generated headers");
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
    let mut text = Text::new("email").with_help_message("Used in generated headers");
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
    inquire::validator::ValueRequiredValidator::new("must not be empty")
}

fn label_index(choice: &str, labels: &[&str]) -> Result<usize, ErrorEnvelope> {
    labels.iter().position(|l| *l == choice).ok_or_else(|| {
        ErrorEnvelope::user_error(
            "invalid wizard selection",
            None,
            Some(choice),
            "choose a listed option",
        )
    })
}

fn inquire_to_user_error(err: InquireError) -> ErrorEnvelope {
    let (msg, value) = match &err {
        InquireError::OperationCanceled | InquireError::OperationInterrupted => {
            ("setup wizard cancelled".to_string(), None::<String>)
        }
        other => (other.to_string(), None::<String>),
    };
    ErrorEnvelope::user_error(msg, None, value.as_deref(), "re-run setup or use flags")
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

// Silence unused-import diagnostics when `EnabledTypes`/`git_info` are only
// referenced from wizard runtime paths.
#[allow(dead_code)]
fn _enabled_types_assert(_t: EnabledTypes) {}
