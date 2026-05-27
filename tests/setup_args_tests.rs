//! Task 1: setup CLI flag surface (D-08, CONF-08, FOUND-04).
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use crud_cli::cli::args::try_parse_cli;
use crud_cli::cli::args::{
    SetupBackend, SetupComponentLibrary, SetupFrontend, SetupOverwritePolicy,
};
use crud_cli::core::error::Kind;
use std::process::Command;

fn full_setup_argv() -> Vec<&'static str> {
    vec![
        "crud-cli",
        "setup",
        "--backend",
        "spring-boot",
        "--frontend",
        "vue",
        "--component-library",
        "element-plus",
        "--overwrite-policy",
        "never",
    ]
}

#[test]
fn setup_args_accept_valid_enum() {
    let cli = try_parse_cli(full_setup_argv()).expect("valid enums parse");
    let setup = match cli.command {
        Some(crud_cli::cli::Commands::Setup(s)) => s,
        _ => panic!("expected setup subcommand"),
    };
    assert!(matches!(setup.backend, Some(SetupBackend::SpringBoot)));
    assert!(matches!(setup.frontend, Some(SetupFrontend::Vue)));
    assert!(matches!(
        setup.component_library,
        Some(SetupComponentLibrary::ElementPlus)
    ));
    assert!(matches!(
        setup.overwrite_policy,
        Some(SetupOverwritePolicy::Never)
    ));
}

#[test]
fn setup_args_reject_invalid_enum() {
    let mut argv = full_setup_argv();
    argv[3] = "django";
    let err = try_parse_cli(argv).expect_err("invalid backend");
    assert_eq!(err.kind, Kind::UserError);
    assert_eq!(err.exit_code, 1);
    assert!(
        err.details.get("flag").is_some() || err.msg.contains("backend"),
        "expected flag detail: {:?}",
        err.details
    );
}

#[test]
fn setup_args_force_flag_surface() {
    let mut argv = full_setup_argv();
    argv.push("--force");
    let cli = try_parse_cli(argv).expect("parse with --force");
    let setup = match cli.command {
        Some(crud_cli::cli::Commands::Setup(s)) => s,
        _ => panic!("setup"),
    };
    assert!(setup.force);
}

#[test]
fn cli_help_version_smoke() {
    use clap::CommandFactory;
    let mut cmd = crud_cli::cli::Cli::command();
    let root_help = cmd.render_help().to_string();
    assert!(root_help.contains("setup"));

    let setup_cmd = cmd
        .find_subcommand_mut("setup")
        .expect("setup subcommand");
    let setup_help = setup_cmd.render_help().to_string();
    assert!(setup_help.contains("--backend"));
    assert!(setup_help.contains("--component-library"));
    assert!(setup_help.contains("--overwrite-policy"));
    assert!(setup_help.contains("--force"));

    assert!(
        cmd.get_version().is_some(),
        "root CLI exposes --version via clap"
    );
    if let Ok(exe) = std::env::var("CARGO_BIN_EXE_crud-cli") {
        let version = Command::new(&exe)
            .arg("--version")
            .output()
            .expect("run --version");
        assert!(version.status.success());
        let ver_str = String::from_utf8_lossy(&version.stdout);
        assert!(ver_str.contains("crud-cli") || ver_str.contains('0'));
    }
}
