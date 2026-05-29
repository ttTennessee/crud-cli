//! GEN-01 `--output` and GEN-07 `java_base` layer-3 fallback (gap closure 02-05).
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use crud_cli::cli::args::GenArgs;
use crud_cli::cli::commands::gen::run_gen;
use crud_cli::core::config::SetupConfig;
use crud_cli::core::config::SetupSelections;
use crud_cli::core::config::{Backend, Frontend};
use std::fs;
use std::sync::{Mutex, OnceLock};
use tempfile::TempDir;

static CWD_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

fn cwd_guard() -> std::sync::MutexGuard<'static, ()> {
    CWD_LOCK.get_or_init(|| Mutex::new(())).lock().unwrap()
}

#[test]
fn gen_output_flag_writes_under_override_root() {
    let dir = TempDir::new().unwrap();
    let root = dir.path();
    let crud = root.join(".crud");
    fs::create_dir_all(crud.join("templates")).unwrap();
    fs::write(crud.join("templates/out.txt.hbs"), "{{model}}\n").unwrap();
    let cfg = SetupConfig::from_selections(SetupSelections {
        backend: Backend::None,
        frontend: Frontend::None,
        template: None,
    });
    fs::write(crud.join("setup.toml"), cfg.to_toml_pretty().unwrap()).unwrap();

    let _lock = cwd_guard();
    let prev = std::env::current_dir().unwrap();
    std::env::set_current_dir(root).unwrap();
    let code = run_gen(GenArgs {
        name: Some("User".into()),
        fields: Some("id:Long".into()),
        package: Some("com.x".into()),
        table: Some("u".into()),
        file: None,
        type_: None,
        dry_run: false,
        stdout: false,
        force: false,
        output: Some(std::path::PathBuf::from("generated")),
        var: vec![],
    });
    std::env::set_current_dir(prev).unwrap();

    assert_eq!(code, 0);
    assert!(root.join("generated/out.txt").is_file());
    assert!(!root.join("out.txt").exists());
}

#[test]
fn gen_java_base_fallback_without_output_flag() {
    let dir = TempDir::new().unwrap();
    let root = dir.path();
    let crud = root.join(".crud");
    fs::create_dir_all(crud.join("templates/java")).unwrap();
    fs::write(crud.join("templates/java/Entity.java.hbs"), "{{model}}\n").unwrap();
    let cfg = SetupConfig::from_selections(SetupSelections {
        backend: Backend::Java,
        frontend: Frontend::None,
        template: None,
    });
    fs::write(crud.join("setup.toml"), cfg.to_toml_pretty().unwrap()).unwrap();

    let _lock = cwd_guard();
    let prev = std::env::current_dir().unwrap();
    std::env::set_current_dir(root).unwrap();
    let code = run_gen(GenArgs {
        name: Some("User".into()),
        fields: Some("id:Long".into()),
        package: Some("com.x".into()),
        table: Some("u".into()),
        file: None,
        type_: None,
        dry_run: false,
        stdout: false,
        force: false,
        output: None,
        var: vec![],
    });
    std::env::set_current_dir(prev).unwrap();

    assert_eq!(code, 0);
    assert!(root.join("src/main/java/Entity.java").is_file());
    assert!(!root.join("java/Entity.java").exists());
}
