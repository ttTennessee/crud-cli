//! `gen_pipeline::resolve_output_path` and orchestration tests.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use crud_cli::core::config::SetupConfig;
use crud_cli::core::config::SetupSelections;
use crud_cli::core::config::{Backend, ComponentLibrary, Frontend, OverwritePolicy};
use crud_cli::core::error::Kind;
use crud_cli::core::gen_pipeline::{resolve_output_path, run};
use crud_cli::core::gen_run::GenRunParams;
use crud_cli::core::template_loader::TemplateEntry;
use std::fs;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};
use tempfile::TempDir;

static CWD_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

fn cwd_guard() -> std::sync::MutexGuard<'static, ()> {
    CWD_LOCK.get_or_init(|| Mutex::new(())).lock().unwrap()
}

fn entry(rel: &str, root: &std::path::Path) -> TemplateEntry {
    let rel_path = PathBuf::from(rel);
    TemplateEntry {
        abs_path: root.join(".crud/templates").join(&rel_path),
        rel_path,
    }
}

#[test]
fn resolve_strips_hbs_suffix() {
    let dir = TempDir::new().expect("tempdir");
    let root = dir.path();
    let e = entry("Entity.java.hbs", root);
    let out = resolve_output_path(&e, root).expect("ok");
    assert_eq!(out, root.join("Entity.java"));
}

#[test]
fn resolve_nested_template_path() {
    let dir = TempDir::new().expect("tempdir");
    let root = dir.path();
    let e = entry("java/Entity.java.hbs", root);
    let out = resolve_output_path(&e, root).expect("ok");
    assert_eq!(out, root.join("java/Entity.java"));
}

#[test]
fn path_traversal_rejected() {
    let dir = TempDir::new().expect("tempdir");
    let root = dir.path();
    let e = entry("../escape.txt", root);
    let err = resolve_output_path(&e, root).expect_err("traversal");
    assert_eq!(err.kind, Kind::UserError);
    assert_eq!(
        err.details.get("reason").and_then(|v| v.as_str()),
        Some("path_traversal")
    );
}

#[test]
fn absolute_rel_path_rejected() {
    let dir = TempDir::new().expect("tempdir");
    let root = dir.path();
    let abs = if cfg!(windows) {
        "C:\\Windows\\evil.hbs"
    } else {
        "/etc/passwd"
    };
    let e = entry(abs, root);
    let err = resolve_output_path(&e, root).expect_err("absolute");
    assert_eq!(
        err.details.get("reason").and_then(|v| v.as_str()),
        Some("path_traversal")
    );
}

#[test]
fn resolve_preserves_non_hbs_extension() {
    let dir = TempDir::new().expect("tempdir");
    let root = dir.path();
    let e = entry("README.md", root);
    let out = resolve_output_path(&e, root).expect("ok");
    assert_eq!(out, root.join("README.md"));
}

fn seed_project(root: &std::path::Path, template_body: &str) {
    let crud = root.join(".crud");
    fs::create_dir_all(crud.join("templates")).unwrap();
    let cfg = SetupConfig::from_selections(SetupSelections {
        backend: Backend::SpringBoot,
        frontend: Frontend::Vue,
        component_library: ComponentLibrary::ElementPlus,
        overwrite_policy: OverwritePolicy::Never,
    });
    fs::write(crud.join("setup.toml"), cfg.to_toml_pretty().unwrap()).unwrap();
    fs::write(crud.join("templates/Entity.java.hbs"), template_body).unwrap();
}

#[test]
fn pipeline_run_writes_rendered_file() {
    let dir = TempDir::new().expect("tempdir");
    let root = dir.path();
    let body = "model={{model_pascal}} pkg={{package}} tbl={{table}}\n{{#each fields}}{{name}}:{{type}}\n{{/each}}List<String>\n";
    seed_project(root, body);

    let _lock = cwd_guard();
    let prev = std::env::current_dir().expect("cwd");
    std::env::set_current_dir(root).unwrap();
    let report = run(GenRunParams {
        name: "User".into(),
        fields_src: "id:Long,name:String".into(),
        package: "com.acme.demo".into(),
        table: "sys_user".into(),
        file: None,
        type_filter: None,
        dry_run: false,
        force: false,
    })
    .expect("run");
    std::env::set_current_dir(&prev).unwrap();

    assert_eq!(report.written.len(), 1);
    assert_eq!(report.written[0], root.join("Entity.java"));
    let content = fs::read_to_string(root.join("Entity.java")).unwrap();
    assert!(content.contains("User"));
    assert!(content.contains("com.acme.demo"));
    assert!(content.contains("sys_user"));
    assert!(content.contains("List<String>"));
    assert!(!content.contains("&lt;"));
}

#[test]
fn dry_run_writes_nothing() {
    let dir = TempDir::new().expect("tempdir");
    let root = dir.path();
    seed_project(root, "{{model}}");

    let _lock = cwd_guard();
    let prev = std::env::current_dir().expect("cwd");
    std::env::set_current_dir(root).unwrap();
    let report = run(GenRunParams {
        name: "User".into(),
        fields_src: "id:Long".into(),
        package: "com.x".into(),
        table: "u".into(),
        file: None,
        type_filter: None,
        dry_run: true,
        force: false,
    })
    .expect("dry-run");
    std::env::set_current_dir(&prev).unwrap();

    assert!(report.written.is_empty());
    assert_eq!(report.skipped.len(), 1);
    assert!(!root.join("Entity.java").exists());
}
