//! `_variables.toml` schema + `--var` end-to-end behavior.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use crud_cli::cli::args::GenArgs;
use crud_cli::cli::commands::gen::run_gen;
use crud_cli::core::config::{
    Backend, Frontend, SetupConfig, SetupSelections,
};
use std::fs;
use std::path::Path;
use std::sync::{Mutex, OnceLock};
use tempfile::TempDir;

static CWD_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

fn cwd_guard() -> std::sync::MutexGuard<'static, ()> {
    CWD_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|e| e.into_inner())
}

fn seed(root: &Path, schema: &str, template_body: &str) {
    let crud = root.join(".crud");
    let templates = crud.join("templates");
    fs::create_dir_all(&templates).unwrap();
    let cfg = SetupConfig::from_selections(SetupSelections {
        backend: Backend::None,
        frontend: Frontend::None,
        template: None,
    });
    fs::write(crud.join("setup.toml"), cfg.to_toml_pretty().unwrap()).unwrap();
    if !schema.is_empty() {
        fs::write(templates.join("_variables.toml"), schema).unwrap();
    }
    fs::write(templates.join("Out.txt.hbs"), template_body).unwrap();
}

fn run_in(root: &Path, args: GenArgs) -> i32 {
    let _lock = cwd_guard();
    let prev = std::env::current_dir().unwrap();
    std::env::set_current_dir(root).unwrap();
    let code = run_gen(args);
    std::env::set_current_dir(prev).unwrap();
    code
}

fn base_args() -> GenArgs {
    GenArgs {
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
    }
}

#[test]
fn cli_var_overrides_default() {
    let dir = TempDir::new().unwrap();
    let root = dir.path();
    seed(
        root,
        "[has_import]\ndescription = \"toggle\"\ntype = \"bool\"\ndefault = false\n",
        "import={{has_import}}\n",
    );
    let mut args = base_args();
    args.var.push("has_import=true".into());
    assert_eq!(run_in(root, args), 0);
    let out = fs::read_to_string(root.join("Out.txt")).unwrap();
    assert!(out.contains("import=true"), "got: {out}");
}

#[test]
fn default_used_when_not_overridden() {
    let dir = TempDir::new().unwrap();
    let root = dir.path();
    seed(
        root,
        "[has_import]\ndescription = \"toggle\"\ntype = \"bool\"\ndefault = false\n",
        "import={{has_import}}\n",
    );
    assert_eq!(run_in(root, base_args()), 0);
    let out = fs::read_to_string(root.join("Out.txt")).unwrap();
    assert!(out.contains("import=false"), "got: {out}");
}

#[test]
fn missing_required_fails() {
    let dir = TempDir::new().unwrap();
    let root = dir.path();
    seed(
        root,
        "[entity_caption]\ndescription = \"caption\"\ntype = \"string\"\nrequired = true\n",
        "comment={{entity_caption}}\n",
    );
    assert_ne!(run_in(root, base_args()), 0);
}

#[test]
fn undeclared_var_rejected() {
    let dir = TempDir::new().unwrap();
    let root = dir.path();
    seed(
        root,
        "[has_import]\ndescription = \"toggle\"\ntype = \"bool\"\ndefault = false\n",
        "import={{has_import}}\n",
    );
    let mut args = base_args();
    args.var.push("not_declared=true".into());
    assert_ne!(run_in(root, args), 0);
}

#[test]
fn type_mismatch_rejected() {
    let dir = TempDir::new().unwrap();
    let root = dir.path();
    seed(
        root,
        "[has_import]\ndescription = \"toggle\"\ntype = \"bool\"\n",
        "import={{has_import}}\n",
    );
    let mut args = base_args();
    args.var.push("has_import=hello".into());
    assert_ne!(run_in(root, args), 0);
}

#[test]
fn schema_file_not_rendered_as_template() {
    let dir = TempDir::new().unwrap();
    let root = dir.path();
    seed(
        root,
        "[has_import]\ndescription = \"toggle\"\ntype = \"bool\"\ndefault = false\n",
        "x={{model}}\n",
    );
    assert_eq!(run_in(root, base_args()), 0);
    assert!(!root.join("_variables.toml").exists());
    assert!(!root.join("_variables").exists());
}

#[test]
fn json_variables_override_default() {
    let dir = TempDir::new().unwrap();
    let root = dir.path();
    seed(
        root,
        "[has_import]\ndescription = \"toggle\"\ntype = \"bool\"\ndefault = false\n",
        "import={{has_import}}\n",
    );
    let json_path = root.join("entity.json");
    fs::write(
        &json_path,
        r#"{
  "name": "User",
  "table": "u",
  "package": "com.x",
  "fields": [{"name":"id","type":"Long","is_pk":true}],
  "variables": {"has_import": true}
}"#,
    )
    .unwrap();
    let mut args = base_args();
    args.fields = None;
    args.name = None;
    args.package = None;
    args.table = None;
    args.file = Some(json_path);
    assert_eq!(run_in(root, args), 0);
    let out = fs::read_to_string(root.join("Out.txt")).unwrap();
    assert!(out.contains("import=true"), "got: {out}");
}

#[test]
fn cli_overrides_json() {
    let dir = TempDir::new().unwrap();
    let root = dir.path();
    seed(
        root,
        "[has_import]\ndescription = \"toggle\"\ntype = \"bool\"\ndefault = false\n",
        "import={{has_import}}\n",
    );
    let json_path = root.join("entity.json");
    fs::write(
        &json_path,
        r#"{
  "name": "User",
  "table": "u",
  "package": "com.x",
  "fields": [{"name":"id","type":"Long","is_pk":true}],
  "variables": {"has_import": false}
}"#,
    )
    .unwrap();
    let mut args = base_args();
    args.fields = None;
    args.name = None;
    args.package = None;
    args.table = None;
    args.file = Some(json_path);
    args.var.push("has_import=true".into());
    assert_eq!(run_in(root, args), 0);
    let out = fs::read_to_string(root.join("Out.txt")).unwrap();
    assert!(out.contains("import=true"), "got: {out}");
}
