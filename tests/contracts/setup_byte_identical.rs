//! D-10: flag and wizard paths produce byte-identical TOML.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use crud_cli::cli::args::{
    SetupArgs, SetupBackend, SetupComponentLibrary, SetupFrontend, SetupOverwritePolicy,
};
use crud_cli::cli::setup_wizard::selections_from_answers;
use crud_cli::core::config::SetupConfig;

#[test]
fn setup_byte_identical() {
    let args = SetupArgs {
        backend: Some(SetupBackend::SpringBoot),
        frontend: Some(SetupFrontend::Vue),
        component_library: Some(SetupComponentLibrary::ElementPlus),
        overwrite_policy: Some(SetupOverwritePolicy::Never),
        force: false,
    };
    let from_flags = args.to_setup_config().expect("flags");
    let from_wizard = SetupConfig::from_selections(selections_from_answers(
        SetupBackend::SpringBoot,
        SetupFrontend::Vue,
        SetupComponentLibrary::ElementPlus,
        SetupOverwritePolicy::Never,
    ));
    assert_eq!(
        from_flags.to_toml_pretty().expect("flags toml"),
        from_wizard.to_toml_pretty().expect("wizard toml")
    );
}
