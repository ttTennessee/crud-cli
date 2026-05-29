//! `gen_pipeline::resolve_output_path` and orchestration tests.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use crud_cli::core::config::SetupConfig;
use crud_cli::core::config::SetupSelections;
use crud_cli::core::config::{Backend, Frontend};
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
    CWD_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|e| e.into_inner())
}

fn entry(rel: &str, root: &std::path::Path) -> TemplateEntry {
    let rel_path = PathBuf::from(rel);
    TemplateEntry {
        abs_path: root.join(".crud/templates").join(&rel_path),
        rel_path,
    }
}

fn setup_none() -> SetupConfig {
    SetupConfig::from_selections(SetupSelections {
        backend: Backend::None,
        frontend: Frontend::None,
        template: None,
    })
}

fn setup_spring_boot() -> SetupConfig {
    SetupConfig::from_selections(SetupSelections {
        backend: Backend::Java,
        frontend: Frontend::None,
        template: None,
    })
}

#[test]
fn resolve_strips_hbs_suffix() {
    let dir = TempDir::new().expect("tempdir");
    let root = dir.path();
    let setup = setup_none();
    let e = entry("Entity.java.hbs", root);
    let out = resolve_output_path(
        &e,
        &crud_cli::core::template_meta::TemplateMeta::default(),
        &crud_cli::core::config::OutputsSection::default(),
        &serde_json::json!({}),
        root,
        None,
        &setup,
    )
    .expect("ok");
    assert_eq!(out, root.join("Entity.java"));
}

#[test]
fn resolve_nested_template_path() {
    let dir = TempDir::new().expect("tempdir");
    let root = dir.path();
    let setup = setup_none();
    let e = entry("java/Entity.java.hbs", root);
    let out = resolve_output_path(
        &e,
        &crud_cli::core::template_meta::TemplateMeta::default(),
        &crud_cli::core::config::OutputsSection::default(),
        &serde_json::json!({}),
        root,
        None,
        &setup,
    )
    .expect("ok");
    assert_eq!(out, root.join("java/Entity.java"));
}

#[test]
fn resolve_java_base_strips_prefix() {
    let dir = TempDir::new().expect("tempdir");
    let root = dir.path();
    let setup = setup_spring_boot();
    let e = entry("java/Entity.java.hbs", root);
    let out = resolve_output_path(
        &e,
        &crud_cli::core::template_meta::TemplateMeta::default(),
        &crud_cli::core::config::OutputsSection::default(),
        &serde_json::json!({}),
        root,
        None,
        &setup,
    )
    .expect("ok");
    assert_eq!(out, root.join("src/main/java/Entity.java"));
}

#[test]
fn resolve_java_base_applied_with_filename_front_matter() {
    let dir = TempDir::new().expect("tempdir");
    let root = dir.path();
    let setup = setup_spring_boot();
    let e = entry("java/Entity.java.hbs", root);
    let meta = crud_cli::core::template_meta::TemplateMeta {
        filename: Some("Renamed.java".into()),
        ..Default::default()
    };
    let out = resolve_output_path(
        &e,
        &meta,
        &crud_cli::core::config::OutputsSection::default(),
        &serde_json::json!({}),
        root,
        None,
        &setup,
    )
    .expect("ok");
    assert_eq!(out, root.join("src/main/java/Renamed.java"));
}

#[test]
fn resolve_java_base_applied_with_base_path_front_matter() {
    let dir = TempDir::new().expect("tempdir");
    let root = dir.path();
    let setup = setup_spring_boot();
    let e = entry("java/Entity.java.hbs", root);
    let meta = crud_cli::core::template_meta::TemplateMeta {
        base_path: Some("java/com/acme/controller".into()),
        filename: Some("Entity.java".into()),
        ..Default::default()
    };
    let out = resolve_output_path(
        &e,
        &meta,
        &crud_cli::core::config::OutputsSection::default(),
        &serde_json::json!({}),
        root,
        None,
        &setup,
    )
    .expect("ok");
    assert_eq!(
        out,
        root.join("src/main/java/com/acme/controller/Entity.java")
    );
}

#[test]
fn resolve_output_override_root() {
    let dir = TempDir::new().expect("tempdir");
    let root = dir.path();
    let setup = setup_none();
    let e = entry("out.txt.hbs", root);
    let out = resolve_output_path(
        &e,
        &crud_cli::core::template_meta::TemplateMeta::default(),
        &crud_cli::core::config::OutputsSection::default(),
        &serde_json::json!({}),
        root,
        Some(PathBuf::from("generated").as_path()),
        &setup,
    )
    .expect("ok");
    assert_eq!(out, root.join("generated/out.txt"));
}

#[test]
fn path_traversal_rejected() {
    let dir = TempDir::new().expect("tempdir");
    let root = dir.path();
    let e = entry("../escape.txt", root);
    let setup = setup_none();
    let err = resolve_output_path(
        &e,
        &crud_cli::core::template_meta::TemplateMeta::default(),
        &crud_cli::core::config::OutputsSection::default(),
        &serde_json::json!({}),
        root,
        None,
        &setup,
    )
    .expect_err("traversal");
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
    let setup = setup_none();
    let err = resolve_output_path(
        &e,
        &crud_cli::core::template_meta::TemplateMeta::default(),
        &crud_cli::core::config::OutputsSection::default(),
        &serde_json::json!({}),
        root,
        None,
        &setup,
    )
    .expect_err("absolute");
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
    let setup = setup_none();
    let out = resolve_output_path(
        &e,
        &crud_cli::core::template_meta::TemplateMeta::default(),
        &crud_cli::core::config::OutputsSection::default(),
        &serde_json::json!({}),
        root,
        None,
        &setup,
    )
    .expect("ok");
    assert_eq!(out, root.join("README.md"));
}

fn seed_project(root: &std::path::Path, template_body: &str) {
    let crud = root.join(".crud");
    fs::create_dir_all(crud.join("templates")).unwrap();
    let cfg = SetupConfig::from_selections(SetupSelections {
        backend: Backend::Java,
        frontend: Frontend::Vue,
        template: None,
    });
    fs::write(crud.join("setup.toml"), cfg.to_toml_pretty().unwrap()).unwrap();
    fs::write(crud.join("templates/Entity.java.hbs"), template_body).unwrap();
}

#[test]
fn pipeline_run_writes_rendered_file() {
    let dir = TempDir::new().expect("tempdir");
    let root = dir.path().canonicalize().expect("canonicalize");
    let root = root.as_path();
    let body = "model={{model_pascal}} pkg={{package}} tbl={{table}}\n{{#each fields}}{{name}}:{{type}}\n{{/each}}List<String>\n";
    seed_project(root, body);

    let _lock = cwd_guard();
    let prev = std::env::current_dir().expect("cwd");
    std::env::set_current_dir(root).unwrap();
    let report = run(GenRunParams {
        name: Some("User".into()),
        fields_src: Some("id:Long,name:String".into()),
        package: Some("com.acme.demo".into()),
        table: Some("sys_user".into()),
        file: None,
        type_filter: None,
        dry_run: false,
        force: false,
        output_dir: None,
        cli_vars: std::collections::BTreeMap::new(),
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
        name: Some("User".into()),
        fields_src: Some("id:Long".into()),
        package: Some("com.x".into()),
        table: Some("u".into()),
        file: None,
        type_filter: None,
        dry_run: true,
        force: false,
        output_dir: None,
        cli_vars: std::collections::BTreeMap::new(),
    })
    .expect("dry-run");
    std::env::set_current_dir(&prev).unwrap();

    assert!(report.written.is_empty());
    assert_eq!(report.skipped.len(), 1);
    assert!(!root.join("Entity.java").exists());
}
