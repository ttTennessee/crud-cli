//! Task 2: SetupConfig / SetupUserConfig merge and serialization (D-10, CONF-03..05).
#![allow(clippy::unwrap_used, clippy::expect_used)]

use crud_cli::cli::args::{
    SetupArgs, SetupBackend, SetupComponentLibrary, SetupFrontend,
};
use crud_cli::core::config::{
    load_setup_file, Backend, ComponentLibrary, Frontend, SetupConfig, SetupFlagOverlay,
    SetupSelections,
};
use crud_cli::core::error::Kind;
use tempfile::NamedTempFile;

#[test]
fn setup_config_flag_serialization() {
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
    let cfg = args.to_setup_config().expect("config");
    let toml = cfg.to_toml_pretty().expect("toml");
    assert!(toml.contains("[project]"));
    assert!(toml.contains("backend = \"spring-boot\""));
    assert!(toml.contains("java_base = \"src/main/java\""));
    assert!(toml.contains("vue_base = \"src/views\""));
    // Project file no longer carries overwrite policy.
    assert!(!toml.contains("[overwrite]"));
}

#[test]
fn setup_config_merge_precedence() {
    let file_cfg = SetupConfig::from_selections(SetupSelections {
        backend: Backend::Nest,
        frontend: Frontend::None,
        component_library: ComponentLibrary::None,
    });
    let merged = SetupConfig::merge(
        SetupConfig::default_selections(),
        Some(&file_cfg),
        SetupFlagOverlay {
            backend: Some(Backend::SpringBoot),
            frontend: None,
            component_library: None,
            overwrite_policy: None,
            enabled_types: None,
            type_map_fallback: None,
        },
    );
    assert_eq!(merged.project.backend, Backend::SpringBoot);
    assert_eq!(merged.project.frontend, Frontend::None);
    assert_eq!(
        merged.paths.java_base.as_deref(),
        Some("src/main/java")
    );
}

#[test]
fn setup_config_reject_unknown_fields() {
    let mut f = NamedTempFile::new().expect("temp");
    use std::io::Write;
    writeln!(
        f,
        r#"
[project]
backend = "none"
frontend = "none"
component-library = "none"
unknown_key = true

[paths]
"#
    )
    .expect("write");
    let err = load_setup_file(f.path()).expect_err("unknown field");
    assert_eq!(err.kind, Kind::ConfigError);
}

#[test]
fn setup_config_framework_path_defaults() {
    let cfg = SetupConfig::from_selections(SetupSelections {
        backend: Backend::Nest,
        frontend: Frontend::React,
        component_library: ComponentLibrary::Antd,
    });
    assert_eq!(cfg.paths.nest_base.as_deref(), Some("src"));
    assert_eq!(cfg.paths.react_base.as_deref(), Some("src/views"));
    assert!(cfg.paths.java_base.is_none());
    assert!(cfg.paths.vue_base.is_none());
}

#[test]
fn type_map_fallback_flag_error_serializes() {
    let args = SetupArgs {
        project: true,
        backend: Some(SetupBackend::SpringBoot),
        frontend: Some(SetupFrontend::Vue),
        component_library: Some(SetupComponentLibrary::ElementPlus),
        overwrite_policy: None,
        enabled_types: None,
        user_name: None,
        user_email: None,
        type_map_fallback: Some("error".into()),
        force: false,
    };
    let cfg = args.to_setup_config().expect("config");
    let toml = cfg.to_toml_pretty().expect("toml");
    assert!(toml.contains("[type_map]"));
    assert!(toml.contains("fallback = \"error\""));
}

#[test]
fn type_map_fallback_flag_literal_serializes() {
    let args = SetupArgs {
        project: true,
        backend: Some(SetupBackend::Nest),
        frontend: Some(SetupFrontend::React),
        component_library: Some(SetupComponentLibrary::None),
        overwrite_policy: None,
        enabled_types: None,
        user_name: None,
        user_email: None,
        type_map_fallback: Some("any".into()),
        force: false,
    };
    let cfg = args.to_setup_config().expect("config");
    let toml = cfg.to_toml_pretty().expect("toml");
    assert!(toml.contains("fallback = \"any\""));
}

#[test]
fn type_map_fallback_default_passthrough_omitted() {
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
    let cfg = args.to_setup_config().expect("config");
    let toml = cfg.to_toml_pretty().expect("toml");
    assert!(!toml.contains("[type_map]"));
}
