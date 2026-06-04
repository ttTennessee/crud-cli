//! user.enabled-types acts as an implicit --type filter for `gen`; explicit
//! `--type` overrides.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use crud_cli::core::config::{
    Backend, EnabledTypes, Frontend, OverwritePolicy, SetupConfig, SetupSelections,
    SetupUserConfig, UserSelections,
};
use crud_cli::core::gen_pipeline::run;
use crud_cli::core::gen_run::GenRunParams;
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

fn seed(root: &std::path::Path, enabled: EnabledTypes) {
    let crud = root.join(".crud");
    fs::create_dir_all(crud.join("templates/java")).unwrap();
    fs::create_dir_all(crud.join("templates/vue")).unwrap();
    let body = "model={{model_pascal}}\n";
    fs::write(crud.join("templates/java/Entity.java.hbs"), body).unwrap();
    fs::write(crud.join("templates/vue/Entity.vue.hbs"), body).unwrap();
    let project = SetupConfig::from_selections(SetupSelections {
        backend: Backend::Java,
        frontend: Frontend::Vue,
        template: None,
    });
    fs::write(crud.join("setup.toml"), project.to_toml_pretty().unwrap()).unwrap();
    let user = SetupUserConfig::from_user_selections(UserSelections {
        name: "Alice".into(),
        email: "a@example.com".into(),
        overwrite_policy: OverwritePolicy::Always,
        enabled_types: enabled,
    });
    fs::write(crud.join("setup.user.toml"), user.to_toml_pretty().unwrap()).unwrap();
}

fn run_in(root: &std::path::Path, type_filter: Option<Vec<String>>) -> Vec<String> {
    let root = root.canonicalize().unwrap();
    let root = root.as_path();
    let _lock = cwd_guard();
    let prev = std::env::current_dir().unwrap();
    std::env::set_current_dir(root).unwrap();
    let report = run(GenRunParams {
        name: Some("User".into()),
        fields_src: Some("id:Long".into()),
        package: Some("com.acme".into()),
        table: Some("sys_user".into()),
        table_comment: None,
        file: None,
        type_filter,
        dry_run: false,
        stdout: false,
        force: true,
        output_dir: None,
        cli_vars: std::collections::BTreeMap::new(),
    })
    .expect("gen run");
    std::env::set_current_dir(prev).unwrap();
    report
        .written
        .into_iter()
        .map(|p| p.strip_prefix(root).unwrap().to_string_lossy().into_owned())
        .collect()
}

#[test]
fn enabled_types_backend_restricts_to_java_prefix() {
    let dir = TempDir::new().unwrap();
    seed(dir.path(), EnabledTypes::Backend);
    let written = run_in(dir.path(), None);
    assert!(
        written.iter().any(|p| p.contains("Entity.java")),
        "{written:?}"
    );
    assert!(
        !written.iter().any(|p| p.contains("Entity.vue")),
        "{written:?}"
    );
}

#[test]
fn enabled_types_frontend_restricts_to_vue_prefix() {
    let dir = TempDir::new().unwrap();
    seed(dir.path(), EnabledTypes::Frontend);
    let written = run_in(dir.path(), None);
    assert!(
        written.iter().any(|p| p.contains("Entity.vue")),
        "{written:?}"
    );
    assert!(
        !written.iter().any(|p| p.contains("Entity.java")),
        "{written:?}"
    );
}

#[test]
fn enabled_types_all_renders_both() {
    let dir = TempDir::new().unwrap();
    seed(dir.path(), EnabledTypes::All);
    let written = run_in(dir.path(), None);
    assert!(
        written.iter().any(|p| p.contains("Entity.java")),
        "{written:?}"
    );
    assert!(
        written.iter().any(|p| p.contains("Entity.vue")),
        "{written:?}"
    );
}

#[test]
fn explicit_type_filter_overrides_enabled_types() {
    let dir = TempDir::new().unwrap();
    seed(dir.path(), EnabledTypes::Backend);
    let written = run_in(dir.path(), Some(vec!["vue".into()]));
    assert!(
        written.iter().any(|p| p.contains("Entity.vue")),
        "{written:?}"
    );
    assert!(
        !written.iter().any(|p| p.contains("Entity.java")),
        "{written:?}"
    );
}
