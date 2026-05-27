//! Interactive `setup` wizard (`inquire`, CONF-01).

use inquire::{Confirm, Select};
use inquire::error::InquireError;

use crate::core::config::{SetupConfig, SetupSelections};
use crate::core::error::ErrorEnvelope;

use super::args::{
    SetupBackend, SetupComponentLibrary, SetupFrontend, SetupOverwritePolicy,
};

/// Runs interactive prompts and returns canonical `SetupConfig` (D-10).
pub fn run_interactive_wizard() -> Result<SetupConfig, ErrorEnvelope> {
    let selections = collect_selections()?;
    Ok(SetupConfig::from_selections(selections))
}

/// Maps wizard answers without TTY — shared with tests.
#[must_use]
pub fn selections_from_answers(
    backend: SetupBackend,
    frontend: SetupFrontend,
    component_library: SetupComponentLibrary,
    overwrite_policy: SetupOverwritePolicy,
) -> SetupSelections {
    SetupSelections {
        backend: backend.into(),
        frontend: frontend.into(),
        component_library: component_library.into(),
        overwrite_policy: overwrite_policy.into(),
    }
}

fn collect_selections() -> Result<SetupSelections, ErrorEnvelope> {
    let backend = prompt_backend()?;
    let frontend = prompt_frontend()?;
    let component_library = prompt_component_library()?;
    let overwrite_policy = prompt_overwrite_policy()?;
    Ok(selections_from_answers(
        backend,
        frontend,
        component_library,
        overwrite_policy,
    ))
}

fn prompt_backend() -> Result<SetupBackend, ErrorEnvelope> {
    let options = [
        SetupBackend::SpringBoot,
        SetupBackend::Nest,
        SetupBackend::None,
    ];
    let labels: Vec<&str> = options
        .iter()
        .map(|o| backend_label(*o))
        .collect();
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
    let labels: Vec<&str> = options
        .iter()
        .map(|o| component_library_label(*o))
        .collect();
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
    let labels: Vec<&str> = options
        .iter()
        .map(|o| overwrite_policy_label(*o))
        .collect();
    let choice = Select::new("overwrite-policy", labels.clone())
        .prompt()
        .map_err(inquire_to_user_error)?;
    Ok(options[label_index(&choice, &labels)?])
}

/// Optional confirm step for destructive flows (reserved for later plans).
#[allow(dead_code)]
fn confirm_force() -> Result<bool, ErrorEnvelope> {
    Confirm::new("Apply --force for force-only overwrite?")
        .with_default(false)
        .prompt()
        .map_err(inquire_to_user_error)
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

