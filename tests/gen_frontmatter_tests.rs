//! Front-matter driven output paths (D-G28 layer 1).
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use crud_cli::cli::args::GenArgs;
use crud_cli::cli::commands::gen::run_gen;
use crud_cli::core::error::Kind;
use crud_cli::core::gen_pipeline::run;
use crud_cli::core::gen_run::GenRunParams;
use std::fs;
use std::io::Write;
use std::sync::{Mutex, OnceLock};
use tempfile::TempDir;

static CWD_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

fn cwd_guard() -> std::sync::MutexGuard<'static, ()> {
    CWD_LOCK.get_or_init(|| Mutex::new(())).lock().unwrap()
}

fn seed_setup(root: &std::path::Path) {
    let crud = root.join(".crud");
    fs::create_dir_all(crud.join("templates")).unwrap();
    let mut setup = std::fs::File::create(crud.join("setup.toml")).unwrap();
    writeln!(
        setup,
        r#"
[project]
backend = "none"
frontend = "none"
component-library = "none"

[paths]

"#
    )
    .unwrap();
}

#[test]
fn frontmatter_base_path_and_filename() {
    let dir = TempDir::new().unwrap();
    let root = dir.path();
    seed_setup(root);
    let body = "---\nbasePath: \"out/{{model_kebab}}\"\nfilename: \"{{model_pascal}}.java\"\n---\n// {{model}}\n";
    fs::write(root.join(".crud/templates/Entity.java.hbs"), body).unwrap();

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
        force: false,
        output: None,
        var: vec![],
    });
    std::env::set_current_dir(prev).unwrap();

    assert_eq!(code, 0);
    let out = root.join("out/user/User.java");
    assert!(out.is_file(), "expected {}", out.display());
}

#[test]
fn filename_with_slash_rejected() {
    let dir = TempDir::new().unwrap();
    let root = dir.path();
    seed_setup(root);
    let body = "---\nfilename: nested/x.java\n---\nbody\n";
    fs::write(root.join(".crud/templates/bad.hbs"), body).unwrap();

    let _lock = cwd_guard();
    let prev = std::env::current_dir().unwrap();
    std::env::set_current_dir(root).unwrap();
    let err = run(GenRunParams {
        name: Some("User".into()),
        fields_src: Some("id:Long".into()),
        package: Some("com.x".into()),
        table: Some("u".into()),
        file: None,
        type_filter: None,
        dry_run: false,
        force: false,
        output_dir: None,
        cli_vars: std::collections::BTreeMap::new(),
    })
    .expect_err("slash");
    std::env::set_current_dir(prev).unwrap();

    assert_eq!(err.kind, Kind::UserError);
    assert_eq!(
        err.details.get("reason").and_then(|v| v.as_str()),
        Some("filename_has_slash")
    );
}

#[test]
fn frontmatter_path_traversal_rejected() {
    let dir = TempDir::new().unwrap();
    let root = dir.path();
    seed_setup(root);
    let body = "---\nbasePath: \"../escape\"\nfilename: x.txt\n---\nbody\n";
    fs::write(root.join(".crud/templates/evil.hbs"), body).unwrap();

    let _lock = cwd_guard();
    let prev = std::env::current_dir().unwrap();
    std::env::set_current_dir(root).unwrap();
    let err = run(GenRunParams {
        name: Some("User".into()),
        fields_src: Some("id:Long".into()),
        package: Some("com.x".into()),
        table: Some("u".into()),
        file: None,
        type_filter: None,
        dry_run: false,
        force: false,
        output_dir: None,
        cli_vars: std::collections::BTreeMap::new(),
    })
    .expect_err("traversal");
    std::env::set_current_dir(prev).unwrap();

    assert_eq!(
        err.details.get("reason").and_then(|v| v.as_str()),
        Some("path_traversal")
    );
}
