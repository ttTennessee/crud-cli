//! Interactive wizard helpers and byte-identical contract.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use crud_cli::cli::args::{SetupArgs, SetupEnabledTypes, SetupOverwritePolicy};
use crud_cli::cli::setup_wizard::user_selections_from_answers;
use crud_cli::core::config::{
    Backend, EnabledTypes, Frontend, OverwritePolicy, SetupConfig, SetupSelections,
};
use crud_cli::core::error::{ErrorEnvelope, Kind};
use inquire::error::InquireError;

fn setup_args(backend: &str, frontend: &str) -> SetupArgs {
    SetupArgs {
        project: true,
        backend: Some(backend.into()),
        frontend: Some(frontend.into()),
        template: None,
        lang: Vec::new(),
        aux: Vec::new(),
        overwrite_policy: None,
        enabled_types: None,
        user_name: None,
        user_email: None,
        type_map_fallback: None,
        force: false,
    }
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
fn project_setup_flag_and_selections_match() {
    let from_flags = setup_args("java", "vue").to_setup_config().expect("flags");
    let from_selections = SetupConfig::from_selections(SetupSelections {
        backend: Backend::Java,
        frontend: Frontend::Vue,
        template: None,
    });
    assert_eq!(
        from_flags.to_toml_pretty().expect("a"),
        from_selections.to_toml_pretty().expect("b")
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
