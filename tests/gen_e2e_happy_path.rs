//! End-to-end happy path for `gen` (Plan 02-01 Task 1 RED → Task 5 GREEN).
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

fn seed_setup(root: &std::path::Path) {
    let crud = root.join(".crud");
    fs::create_dir_all(&crud).expect("mkdir .crud");
    let cfg = SetupConfig::from_selections(SetupSelections {
        backend: Backend::Java,
        frontend: Frontend::Vue,
        template: None,
    });
    let toml = cfg.to_toml_pretty().expect("serialize setup");
    fs::write(crud.join("setup.toml"), toml).expect("write setup.toml");
}

fn seed_template(root: &std::path::Path) {
    let templates = root.join(".crud/templates");
    fs::create_dir_all(&templates).expect("mkdir templates");
    let body = r#"package {{package}};
// model {{model_pascal}} table {{table}}
{{#each fields}}{{name}}:{{type}}
{{/each}}
List<String>
"#;
    fs::write(templates.join("Entity.java.hbs"), body).expect("write template");
}

#[test]
fn gen_renders_single_template_to_disk() {
    let dir = TempDir::new().expect("tempdir");
    let root = dir.path();
    seed_setup(root);
    seed_template(root);

    let _lock = cwd_guard();
    let prev = std::env::current_dir().expect("cwd");
    std::env::set_current_dir(root).expect("chdir temp");
    let code = run_gen(GenArgs {
        name: Some("User".into()),
        fields: Some("id:Long,name:String".into()),
        package: Some("com.acme.demo".into()),
        table: Some("sys_user".into()),
        table_comment: None,
        file: None,
        type_: None,
        dry_run: false,
        stdout: false,
        force: false,
        output: None,
        var: vec![],
    });
    std::env::set_current_dir(&prev).expect("restore cwd");

    assert_eq!(code, 0, "gen should succeed");
    let out = root.join("Entity.java");
    assert!(out.is_file(), "Entity.java should exist at project root");
    let content = fs::read_to_string(&out).expect("read output");
    assert!(content.contains("User"));
    assert!(content.contains("com.acme.demo"));
    assert!(content.contains("sys_user"));
    assert!(content.contains("id:Long"));
    assert!(content.contains("name:String"));
    assert!(content.contains("List<String>"));
    assert!(!content.contains("&lt;"));
}
