//! Project flag and selection-based paths produce byte-identical TOML.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use crud_cli::cli::args::SetupArgs;
use crud_cli::core::config::{Backend, Frontend, SetupConfig, SetupSelections};

#[test]
fn setup_byte_identical() {
    let args = SetupArgs {
        project: true,
        backend: Some("java".into()),
        frontend: Some("vue".into()),
        template: None,
        lang: Vec::new(),
        aux: Vec::new(),
        overwrite_policy: None,
        enabled_types: None,
        user_name: None,
        user_email: None,
        type_map_fallback: None,
        force: false,
    };
    let from_flags = args.to_setup_config().expect("flags");
    let from_selections = SetupConfig::from_selections(SetupSelections {
        backend: Backend::Java,
        frontend: Frontend::Vue,
        template: None,
    });
    assert_eq!(
        from_flags.to_toml_pretty().expect("flags toml"),
        from_selections.to_toml_pretty().expect("selections toml")
    );
}
