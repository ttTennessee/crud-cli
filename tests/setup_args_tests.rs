//! setup CLI flag surface.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use crud_cli::cli::args::{try_parse_cli, SetupOverwritePolicy};
use crud_cli::core::error::Kind;
use std::process::Command;

fn full_setup_argv() -> Vec<&'static str> {
    vec![
        "crud-cli",
        "setup",
        "--backend",
        "java",
        "--frontend",
        "vue",
        "--overwrite-policy",
        "never",
    ]
}

#[test]
fn setup_args_accept_valid_languages() {
    let cli = try_parse_cli(full_setup_argv()).expect("valid languages parse");
    let setup = match cli.command {
        Some(crud_cli::cli::Commands::Setup(s)) => s,
        _ => panic!("expected setup subcommand"),
    };
    assert_eq!(setup.backend.as_deref(), Some("java"));
    assert_eq!(setup.frontend.as_deref(), Some("vue"));
    assert!(matches!(
        setup.overwrite_policy,
        Some(SetupOverwritePolicy::Never)
    ));
}

#[test]
fn setup_args_accept_custom_language() {
    let argv = vec![
        "crud-cli",
        "setup",
        "--backend",
        "php",
        "--frontend",
        "vue",
    ];
    let cli = try_parse_cli(argv).expect("custom backend parses");
    let setup = match cli.command {
        Some(crud_cli::cli::Commands::Setup(s)) => s,
        _ => panic!("expected setup"),
    };
    assert_eq!(setup.backend.as_deref(), Some("php"));
}

#[test]
fn setup_args_reject_invalid_lang_id_at_to_setup_config() {
    let argv = vec![
        "crud-cli",
        "setup",
        "--project",
        "--backend",
        "Spring Boot",
        "--frontend",
        "vue",
    ];
    let cli = try_parse_cli(argv).expect("parses (validation deferred)");
    let setup = match cli.command {
        Some(crud_cli::cli::Commands::Setup(s)) => s,
        _ => panic!("setup"),
    };
    let err = setup.to_setup_config().expect_err("invalid id");
    assert_eq!(err.kind, Kind::UserError);
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
fn setup_args_template_flag() {
    let argv = vec![
        "crud-cli",
        "setup",
        "--project",
        "--backend",
        "java",
        "--frontend",
        "vue",
        "--template",
        "ruoyi@1.0.0",
    ];
    let cli = try_parse_cli(argv).expect("parse");
    let setup = match cli.command {
        Some(crud_cli::cli::Commands::Setup(s)) => s,
        _ => panic!("setup"),
    };
    let cfg = setup.to_setup_config().expect("config");
    assert_eq!(
        cfg.project.template.as_ref().map(ToString::to_string),
        Some("ruoyi@1.0.0".to_string())
    );
}

#[test]
fn cli_help_version_smoke() {
    use clap::CommandFactory;
    let mut cmd = crud_cli::cli::Cli::command();
    let root_help = cmd.render_help().to_string();
    assert!(root_help.contains("setup"));
    assert!(root_help.contains("template"));

    let setup_cmd = cmd
        .find_subcommand_mut("setup")
        .expect("setup subcommand");
    let setup_help = setup_cmd.render_help().to_string();
    assert!(setup_help.contains("--backend"));
    assert!(setup_help.contains("--overwrite-policy"));
    assert!(setup_help.contains("--force"));
    assert!(setup_help.contains("--template"));

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
