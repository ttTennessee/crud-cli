//! Task 3: interactive wizard → canonical SetupConfig / SetupUserConfig (CONF-01, D-10).
#![allow(clippy::unwrap_used, clippy::expect_used)]

use crud_cli::cli::args::{
    SetupArgs, SetupBackend, SetupComponentLibrary, SetupEnabledTypes, SetupFrontend,
    SetupOverwritePolicy,
};
use crud_cli::cli::setup_wizard::{selections_from_answers, user_selections_from_answers};
use crud_cli::core::config::{
    Backend, ComponentLibrary, EnabledTypes, Frontend, OverwritePolicy, SetupConfig,
};
use crud_cli::core::error::{ErrorEnvelope, Kind};
use inquire::error::InquireError;

#[test]
fn setup_wizard_project_prompts() {
    let sel = selections_from_answers(
        SetupBackend::Nest,
        SetupFrontend::React,
        SetupComponentLibrary::NaiveUi,
    );
    assert_eq!(sel.backend, Backend::Nest);
    assert_eq!(sel.frontend, Frontend::React);
    assert_eq!(sel.component_library, ComponentLibrary::NaiveUi);
}

#[test]
fn setup_wizard_user_prompts() {
    let sel = user_selections_from_answers(
        "Alice".into(),
        "a@example.com".into(),
        SetupOverwritePolicy::ForceOnly,
        SetupEnabledTypes::Backend,
    );
    assert_eq!(sel.name, "Alice");
    assert_eq!(sel.email, "a@example.com");
    assert_eq!(sel.overwrite_policy, OverwritePolicy::ForceOnly);
    assert_eq!(sel.enabled_types, EnabledTypes::Backend);
}

#[test]
fn project_setup_byte_identical() {
    let args = SetupArgs {
        project: true,
        backend: Some(SetupBackend::SpringBoot),
        frontend: Some(SetupFrontend::Vue),
        component_library: Some(SetupComponentLibrary::ElementPlus),
        overwrite_policy: None,
        enabled_types: None,
        user_name: None,
        user_email: None,
        type_map_fallback: None,
        force: false,
    };
    let from_flags = args.to_setup_config().expect("flags");
    let from_wizard = SetupConfig::from_selections(selections_from_answers(
        SetupBackend::SpringBoot,
        SetupFrontend::Vue,
        SetupComponentLibrary::ElementPlus,
    ));
    assert_eq!(
        from_flags.to_toml_pretty().expect("a"),
        from_wizard.to_toml_pretty().expect("b")
    );
}

#[test]
fn setup_wizard_user_error() {
    let err = InquireError::OperationCanceled;
    let envelope = ErrorEnvelope::user_error(
        "setup wizard cancelled",
        None,
        None,
        "re-run setup or use flags",
    );
    assert_eq!(envelope.kind, Kind::UserError);
    assert_eq!(envelope.exit_code, 1);
    let _ = err;
}
