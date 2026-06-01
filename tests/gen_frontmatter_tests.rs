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
        table_comment: None,
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
        table_comment: None,
        file: None,
        type_filter: None,
        dry_run: false,
        stdout: false,
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

fn seed_with_import_schema(root: &std::path::Path) {
    seed_setup(root);
    fs::write(
        root.join(".crud/templates/_variables.toml"),
        "[has_import]\ndescription = \"toggle import DTO\"\ntype = \"bool\"\ndefault = false\n",
    )
    .unwrap();
    // Always-generated entity so the run has unconditional output too.
    fs::write(
        root.join(".crud/templates/Entity.java.hbs"),
        "---\nfilename: \"{{model_pascal}}.java\"\n---\n// {{model}}\n",
    )
    .unwrap();
    // Conditional import DTO.
    fs::write(
        root.join(".crud/templates/ImportDTO.java.hbs"),
        "---\ngenerateWhen: has_import\nfilename: \"{{model_pascal}}ImportDTO.java\"\n---\n// import dto\n",
    )
    .unwrap();
}

fn run_with_has_import(root: &std::path::Path, has_import: Option<bool>) -> crud_cli::core::gen_report::GenReport {
    let mut cli_vars = std::collections::BTreeMap::new();
    if let Some(v) = has_import {
        cli_vars.insert("has_import".to_string(), serde_json::Value::Bool(v));
    }
    let _lock = cwd_guard();
    let prev = std::env::current_dir().unwrap();
    std::env::set_current_dir(root).unwrap();
    let report = run(GenRunParams {
        name: Some("User".into()),
        fields_src: Some("id:Long".into()),
        package: Some("com.x".into()),
        table: Some("u".into()),
        table_comment: None,
        file: None,
        type_filter: None,
        dry_run: false,
        stdout: false,
        force: false,
        output_dir: None,
        cli_vars,
    });
    std::env::set_current_dir(prev).unwrap();
    report.expect("gen ok")
}

#[test]
fn generate_when_false_skips_file_and_reports_it() {
    let dir = TempDir::new().unwrap();
    let root = dir.path();
    seed_with_import_schema(root);

    let report = run_with_has_import(root, Some(false));

    assert!(
        !root.join("UserImportDTO.java").exists(),
        "conditional DTO must not be generated when has_import=false"
    );
    assert!(root.join("User.java").is_file(), "entity always generated");
    assert_eq!(report.written.len(), 1);
    // On macOS `TempDir` returns `/var/...` while `current_dir()` after `set_current_dir`
    // resolves to `/private/var/...`; compare canonical paths to stay platform-portable.
    let canon_root = fs::canonicalize(root).unwrap();
    assert_eq!(
        report.skipped_by_condition,
        vec![canon_root.join("UserImportDTO.java")]
    );
}

#[test]
fn generate_when_true_renders_file() {
    let dir = TempDir::new().unwrap();
    let root = dir.path();
    seed_with_import_schema(root);

    let report = run_with_has_import(root, Some(true));

    assert!(
        root.join("UserImportDTO.java").is_file(),
        "conditional DTO must be generated when has_import=true"
    );
    assert!(report.skipped_by_condition.is_empty());
    assert_eq!(report.written.len(), 2);
}

#[test]
fn stdout_mode_renders_without_writing() {
    let dir = TempDir::new().unwrap();
    let root = dir.path();
    seed_setup(root);
    fs::write(
        root.join(".crud/templates/Entity.java.hbs"),
        "---\nfilename: \"{{model_pascal}}.java\"\n---\n// {{model}}\n",
    )
    .unwrap();

    let _lock = cwd_guard();
    let prev = std::env::current_dir().unwrap();
    std::env::set_current_dir(root).unwrap();
    let report = run(GenRunParams {
        name: Some("User".into()),
        fields_src: Some("id:Long".into()),
        package: Some("com.x".into()),
        table: Some("u".into()),
        table_comment: None,
        file: None,
        type_filter: None,
        dry_run: false,
        stdout: true,
        force: false,
        output_dir: None,
        cli_vars: std::collections::BTreeMap::new(),
    })
    .expect("ok");
    std::env::set_current_dir(prev).unwrap();

    assert!(
        !root.join("User.java").exists(),
        "stdout mode must not write files"
    );
    assert!(report.written.is_empty());
    assert_eq!(report.rendered.len(), 1);
    assert!(report.rendered[0].content.contains("// User"));
    assert!(report.rendered[0].path.ends_with("User.java"));
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
        table_comment: None,
        file: None,
        type_filter: None,
        dry_run: false,
        stdout: false,
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
