//! SetupUserConfig round-trip + validation contract.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use crud_cli::core::config::{
    load_user_setup_file, EnabledTypes, OverwritePolicy, SetupUserConfig, UserSelections,
};
use crud_cli::core::error::Kind;
use std::io::Write;
use tempfile::NamedTempFile;

#[test]
fn user_config_serializes_sections_in_order() {
    let cfg = SetupUserConfig::from_user_selections(UserSelections {
        name: "Alice".into(),
        email: "a@example.com".into(),
        overwrite_policy: OverwritePolicy::ForceOnly,
        enabled_types: EnabledTypes::Backend,
    });
    let toml = cfg.to_toml_pretty().expect("serialize");
    let user_idx = toml.find("[user]").expect("user");
    let overwrite_idx = toml.find("[overwrite]").expect("overwrite");
    let scope_idx = toml.find("[scope]").expect("scope");
    assert!(user_idx < overwrite_idx);
    assert!(overwrite_idx < scope_idx);
    assert!(toml.contains("name = \"Alice\""));
    assert!(toml.contains("email = \"a@example.com\""));
    assert!(toml.contains("overwrite-policy = \"force-only\""));
    assert!(toml.contains("enabled-types = \"backend\""));
}

#[test]
fn user_config_load_rejects_empty_name() {
    let mut f = NamedTempFile::new().unwrap();
    writeln!(
        f,
        r#"
[user]
name = ""
email = "a@example.com"

[overwrite]
overwrite-policy = "never"
"#
    )
    .unwrap();
    let err = load_user_setup_file(f.path()).expect_err("empty name");
    assert_eq!(err.kind, Kind::ConfigError);
}

#[test]
fn user_config_load_rejects_unknown_fields() {
    let mut f = NamedTempFile::new().unwrap();
    writeln!(
        f,
        r#"
[user]
name = "Alice"
email = "a@example.com"
nickname = "ali"

[overwrite]
overwrite-policy = "never"
"#
    )
    .unwrap();
    let err = load_user_setup_file(f.path()).expect_err("unknown field");
    assert_eq!(err.kind, Kind::ConfigError);
}

#[test]
fn user_config_load_defaults_scope_when_missing() {
    let mut f = NamedTempFile::new().unwrap();
    writeln!(
        f,
        r#"
[user]
name = "Alice"
email = "a@example.com"

[overwrite]
overwrite-policy = "never"
"#
    )
    .unwrap();
    let cfg = load_user_setup_file(f.path()).expect("load");
    assert_eq!(cfg.scope.enabled_types, EnabledTypes::All);
}
