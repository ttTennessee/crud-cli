//! `[variables]` reserved-name blacklist at setup load (D-G21).
#![allow(clippy::unwrap_used, clippy::expect_used)]

use crud_cli::core::config::load_setup_file;
use crud_cli::core::error::Kind;
use std::io::Write;
use tempfile::NamedTempFile;

#[test]
fn reserved_variable_in_setup_toml() {
    let mut f = NamedTempFile::new().expect("temp");
    writeln!(
        f,
        r#"
[project]
backend = "none"
frontend = "none"
component-library = "none"

[paths]

[overwrite]
overwrite-policy = "never"

[variables]
model = "shadow"
"#
    )
    .unwrap();
    let err = load_setup_file(f.path()).expect_err("reserved");
    assert_eq!(err.kind, Kind::ConfigError);
    assert_eq!(
        err.details.get("reason").and_then(|v| v.as_str()),
        Some("reserved_variable")
    );
    assert_eq!(
        err.details.get("variable").and_then(|v| v.as_str()),
        Some("model")
    );
}
