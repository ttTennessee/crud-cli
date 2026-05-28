//! `--type` template prefix filter (D-G31, D-G32).
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use crud_cli::core::error::Kind;
use crud_cli::core::template_loader::discover_templates;
use std::fs;
use std::sync::{Mutex, OnceLock};
use tempfile::TempDir;

static CWD_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

fn cwd_guard() -> std::sync::MutexGuard<'static, ()> {
    CWD_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|e| e.into_inner())
}

fn seed_templates(root: &std::path::Path) {
    let t = root.join(".crud/templates");
    fs::create_dir_all(t.join("java")).unwrap();
    fs::create_dir_all(t.join("vue")).unwrap();
    fs::write(t.join("java/Entity.java.hbs"), "j").unwrap();
    fs::write(t.join("vue/Page.vue.hbs"), "v").unwrap();
    fs::write(t.join("Notes.txt.hbs"), "n").unwrap();
}

#[test]
fn type_filter_java_only() {
    let dir = TempDir::new().unwrap();
    seed_templates(dir.path());
    let _lock = cwd_guard();
    let prev = std::env::current_dir().unwrap();
    std::env::set_current_dir(dir.path()).unwrap();
    let entries = discover_templates(dir.path(), Some(&["java".to_string()])).unwrap();
    std::env::set_current_dir(prev).unwrap();
    assert_eq!(entries.len(), 1);
    let rp = entries[0].rel_path.to_string_lossy().replace('\\', "/");
    assert!(rp.contains("java/"), "{rp}");
}

#[test]
fn type_filter_java_and_vue() {
    let dir = TempDir::new().unwrap();
    seed_templates(dir.path());
    let _lock = cwd_guard();
    let prev = std::env::current_dir().unwrap();
    std::env::set_current_dir(dir.path()).unwrap();
    let entries = discover_templates(
        dir.path(),
        Some(&["java".to_string(), "vue".to_string()]),
    )
    .unwrap();
    std::env::set_current_dir(prev).unwrap();
    assert_eq!(entries.len(), 2);
}

#[test]
fn unknown_type_lists_available() {
    let dir = TempDir::new().unwrap();
    seed_templates(dir.path());
    let _lock = cwd_guard();
    let prev = std::env::current_dir().unwrap();
    std::env::set_current_dir(dir.path()).unwrap();
    let err = discover_templates(dir.path(), Some(&["python".to_string()])).unwrap_err();
    std::env::set_current_dir(prev).unwrap();
    assert_eq!(err.kind, Kind::UserError);
    assert_eq!(
        err.details.get("reason").and_then(|v| v.as_str()),
        Some("template_type_not_found")
    );
    assert!(err.hint.contains("java"));
    assert!(err.hint.contains("vue"));
}
