//! Task 1: `gen` CLI flag surface (D-G09, D-G10).
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use crud_cli::cli::args::{try_parse_cli, GenArgs, Commands};
use crud_cli::core::error::Kind;

#[test]
fn gen_args_parses_minimal_dsl_form() {
    let cli = try_parse_cli([
        "crud-cli",
        "gen",
        "User",
        "--fields",
        "id:Long",
        "--package",
        "com.acme",
        "--table",
        "u",
    ])
    .expect("valid gen argv");

    let args = match cli.command {
        Some(Commands::Gen(g)) => g,
        _ => panic!("expected gen subcommand"),
    };
    assert_eq!(args.name.as_deref(), Some("User"));
    assert_eq!(args.fields.as_deref(), Some("id:Long"));
    assert_eq!(args.package.as_deref(), Some("com.acme"));
    assert_eq!(args.table.as_deref(), Some("u"));
    assert!(args.file.is_none());
    assert!(!args.dry_run);
    assert!(!args.force);
    assert!(args.output.is_none());
}

#[test]
fn gen_args_parses_output_flag() {
    let cli = try_parse_cli([
        "crud-cli",
        "gen",
        "User",
        "--fields",
        "id:Long",
        "--package",
        "com.x",
        "--table",
        "u",
        "--output",
        "generated",
    ])
    .expect("valid gen argv");

    let args = match cli.command {
        Some(Commands::Gen(g)) => g,
        _ => panic!("expected gen subcommand"),
    };
    assert_eq!(args.output.as_deref(), Some(std::path::Path::new("generated")));
}

#[test]
fn gen_args_fields_file_mutex() {
    let args = GenArgs {
        name: Some("User".into()),
        fields: Some("id:Long".into()),
        file: Some(std::path::PathBuf::from("user.json")),
        package: Some("com.acme".into()),
        table: Some("u".into()),
        type_: None,
        dry_run: false,
        force: false,
        output: None,
        var: vec![],
    };
    let err = args.validate_inputs().expect_err("mutex");
    assert_eq!(err.kind, Kind::UserError);
    assert_eq!(err.exit_code, 1);
    assert_eq!(
        err.details.get("reason").and_then(|v| v.as_str()),
        Some("fields_file_mutex")
    );
}
