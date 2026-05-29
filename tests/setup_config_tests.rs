//! SetupConfig / SetupUserConfig merge and serialization.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use crud_cli::cli::args::SetupArgs;
use crud_cli::core::config::{
    load_setup_file, Backend, Frontend, SetupConfig, SetupFlagOverlay, SetupSelections,
};
use crud_cli::core::error::Kind;
use tempfile::NamedTempFile;

fn args_with_languages(backend: &str, frontend: &str) -> SetupArgs {
    SetupArgs {
        project: true,
        backend: Some(backend.to_string()),
        frontend: Some(frontend.to_string()),
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
fn setup_config_flag_serialization() {
    let cfg = args_with_languages("java", "vue").to_setup_config().expect("config");
    let toml = cfg.to_toml_pretty().expect("toml");
    assert!(toml.contains("[project]"));
    assert!(toml.contains("backend = \"java\""));
    assert!(toml.contains("[paths.lang]"));
    assert!(toml.contains("java = \"src/main/java\""));
    assert!(toml.contains("vue = \"src/views\""));
    assert!(!toml.contains("[overwrite]"));
    assert!(!toml.contains("component"));
}

#[test]
fn setup_config_merge_precedence() {
    let file_cfg = SetupConfig::from_selections(SetupSelections {
        backend: Backend::TypeScript,
        frontend: Frontend::None,
        template: None,
    });
    let merged = SetupConfig::merge(
        SetupConfig::default_selections(),
        Some(&file_cfg),
        SetupFlagOverlay {
            backend: Some(Backend::Java),
            ..Default::default()
        },
    );
    assert_eq!(merged.project.backend, Backend::Java);
    assert_eq!(merged.project.frontend, Frontend::None);
    assert_eq!(
        merged.paths.lang.get("java").map(String::as_str),
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
unknown_key = true

[paths]
"#
    )
    .expect("write");
    let err = load_setup_file(f.path()).expect_err("unknown field");
    assert_eq!(err.kind, Kind::ConfigError);
}

#[test]
fn setup_config_language_path_defaults() {
    let cfg = SetupConfig::from_selections(SetupSelections {
        backend: Backend::TypeScript,
        frontend: Frontend::React,
        template: None,
    });
    assert_eq!(cfg.paths.lang.get("ts").map(String::as_str), Some("src"));
    assert_eq!(
        cfg.paths.lang.get("react").map(String::as_str),
        Some("src/views")
    );
    assert!(!cfg.paths.lang.contains_key("java"));
    assert!(!cfg.paths.lang.contains_key("vue"));
}

#[test]
fn type_map_fallback_flag_error_serializes() {
    let mut args = args_with_languages("java", "vue");
    args.type_map_fallback = Some("error".into());
    let cfg = args.to_setup_config().expect("config");
    let toml = cfg.to_toml_pretty().expect("toml");
    assert!(toml.contains("[type_map]"));
    assert!(toml.contains("fallback = \"error\""));
}

#[test]
fn type_map_fallback_flag_literal_serializes() {
    let mut args = args_with_languages("typescript", "react");
    args.type_map_fallback = Some("any".into());
    let cfg = args.to_setup_config().expect("config");
    let toml = cfg.to_toml_pretty().expect("toml");
    assert!(toml.contains("fallback = \"any\""));
}

#[test]
fn type_map_fallback_default_passthrough_omitted() {
    let args = args_with_languages("java", "vue");
    let cfg = args.to_setup_config().expect("config");
    let toml = cfg.to_toml_pretty().expect("toml");
    assert!(!toml.contains("[type_map]"));
}

#[test]
fn legacy_schema_rejected() {
    let mut f = NamedTempFile::new().expect("temp");
    use std::io::Write;
    writeln!(
        f,
        r#"
[project]
backend = "spring-boot"
frontend = "vue"
component-library = "element-plus"

[paths]
java_base = "src/main/java"
"#
    )
    .expect("write");
    let err = load_setup_file(f.path()).expect_err("legacy");
    assert_eq!(err.kind, Kind::ConfigError);
}
