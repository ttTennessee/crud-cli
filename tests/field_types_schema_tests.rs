//! `_field_types.toml` schema + gen/validate end-to-end behavior.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use crud_cli::cli::args::GenArgs;
use crud_cli::cli::commands::gen::run_gen;
use crud_cli::core::config::{
    Backend, Frontend, SetupConfig, SetupSelections,
};
use crud_cli::core::field_types::SCHEMA_FILE_NAME;
use crud_cli::core::type_map::TYPE_MAP_FILE_NAME;
use crud_cli::core::validator::{run, ValidateParams};
use std::fs;
use std::io::Write;
use std::path::Path;
use std::sync::{Mutex, OnceLock};
use tempfile::{NamedTempFile, TempDir};

static CWD_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

fn cwd_guard() -> std::sync::MutexGuard<'static, ()> {
    CWD_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|e| e.into_inner())
}

fn seed(root: &Path, field_types: &str, template_body: &str) {
    let crud = root.join(".crud");
    let templates = crud.join("templates");
    fs::create_dir_all(&templates).unwrap();
    let cfg = SetupConfig::from_selections(SetupSelections {
        backend: Backend::None,
        frontend: Frontend::None,
        template: None,
    });
    fs::write(crud.join("setup.toml"), cfg.to_toml_pretty().unwrap()).unwrap();
    if !field_types.is_empty() {
        fs::write(templates.join(SCHEMA_FILE_NAME), field_types).unwrap();
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

fn run_validate_in(
    root: &Path,
) -> Result<crud_cli::core::validator::ValidateReport, crud_cli::core::error::ErrorEnvelope> {
    let _lock = cwd_guard();
    let prev = std::env::current_dir().unwrap();
    std::env::set_current_dir(root).unwrap();
    let result = run(ValidateParams::default());
    std::env::set_current_dir(prev).unwrap();
    result
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
fn no_schema_allows_any_type() {
    let dir = TempDir::new().unwrap();
    let root = dir.path();
    seed(root, "", "type={{#each fields}}{{type}}{{/each}}\n");
    let mut args = base_args();
    args.fields = Some("id:CustomType".into());
    assert_eq!(run_in(root, args), 0);
    let out = fs::read_to_string(root.join("Out.txt")).unwrap();
    assert!(out.contains("CustomType"), "got: {out}");
}

#[test]
fn canonical_type_passes() {
    let dir = TempDir::new().unwrap();
    let root = dir.path();
    seed(
        root,
        "[Long]\ndescription = \"64-bit integer\"\n",
        "type={{#each fields}}{{type}}{{/each}}\n",
    );
    assert_eq!(run_in(root, base_args()), 0);
}

#[test]
fn alias_normalized_in_output() {
    let dir = TempDir::new().unwrap();
    let root = dir.path();
    seed(
        root,
        "[Integer]\ndescription = \"32-bit int\"\naliases = [\"int\"]\n",
        "type={{#each fields}}{{type}}{{/each}}\n",
    );
    let mut args = base_args();
    args.fields = Some("age:int".into());
    assert_eq!(run_in(root, args), 0);
    let out = fs::read_to_string(root.join("Out.txt")).unwrap();
    assert!(out.contains("Integer"), "got: {out}");
}

#[test]
fn unknown_type_rejected() {
    let dir = TempDir::new().unwrap();
    let root = dir.path();
    seed(
        root,
        "[Long]\ndescription = \"pk\"\n",
        "x\n",
    );
    let mut args = base_args();
    args.fields = Some("email:String".into());
    assert_ne!(run_in(root, args), 0);
}

#[test]
fn json_unknown_type_rejected() {
    let dir = TempDir::new().unwrap();
    let root = dir.path();
    seed(
        root,
        "[Long]\ndescription = \"pk\"\n",
        "x\n",
    );
    let mut f = NamedTempFile::new().unwrap();
    writeln!(
        f,
        r#"{{
  "name": "User",
  "table": "u",
  "package": "com.x",
  "fields": [{{ "name": "email", "type": "String" }}]
}}"#
    )
    .unwrap();
    let args = GenArgs {
        name: None,
        fields: None,
        package: None,
        table: None,
        table_comment: None,
        file: Some(f.path().to_path_buf()),
        type_: None,
        dry_run: false,
        stdout: false,
        force: false,
        output: None,
        var: vec![],
    };
    assert_ne!(run_in(root, args), 0);
}

#[test]
fn schema_file_not_rendered_as_template() {
    let dir = TempDir::new().unwrap();
    let root = dir.path();
    seed(
        root,
        "[Long]\ndescription = \"pk\"\n",
        "ok\n",
    );
    assert_eq!(run_in(root, base_args()), 0);
    assert!(!root.join("_field_types.toml").exists());
}

#[test]
fn validate_reports_unmapped_type_in_bundles() {
    let dir = TempDir::new().unwrap();
    let root = dir.path();
    let crud = root.join(".crud");
    let templates = crud.join("templates");
    fs::create_dir_all(templates.join("java")).unwrap();
    fs::write(
        crud.join("setup.toml"),
        SetupConfig::from_selections(SetupSelections {
            backend: Backend::Java,
            frontend: Frontend::None,
            template: None,
        })
        .to_toml_pretty()
        .unwrap(),
    )
    .unwrap();
    fs::write(
        templates.join(SCHEMA_FILE_NAME),
        "[Long]\ndescription = \"pk\"\n[String]\ndescription = \"text\"\n",
    )
    .unwrap();
    fs::write(
        templates.join("java").join(TYPE_MAP_FILE_NAME),
        "[map]\nLong = \"Long\"\n",
    )
    .unwrap();
    fs::write(templates.join("Good.hbs"), "x\n").unwrap();

    let err = run_validate_in(root).expect_err("should fail");
    let issues = err.details.get("issues").and_then(|v| v.as_array()).unwrap();
    assert!(
        issues.iter().any(|i| {
            i.get("kind")
                .and_then(|k| k.as_str())
                == Some("field_type_unmapped")
        }),
        "expected field_type_unmapped issue, got: {issues:?}"
    );
}

#[test]
fn validate_passes_when_all_types_mapped() {
    let dir = TempDir::new().unwrap();
    let root = dir.path();
    let crud = root.join(".crud");
    let templates = crud.join("templates");
    fs::create_dir_all(templates.join("java")).unwrap();
    fs::write(
        crud.join("setup.toml"),
        SetupConfig::from_selections(SetupSelections {
            backend: Backend::Java,
            frontend: Frontend::None,
            template: None,
        })
        .to_toml_pretty()
        .unwrap(),
    )
    .unwrap();
    fs::write(
        templates.join(SCHEMA_FILE_NAME),
        "[Long]\ndescription = \"pk\"\n",
    )
    .unwrap();
    fs::write(
        templates.join("java").join(TYPE_MAP_FILE_NAME),
        "[map]\nLong = \"Long\"\n",
    )
    .unwrap();
    fs::write(templates.join("Good.hbs"), "x\n").unwrap();

    let report = run_validate_in(root).expect("should pass");
    assert_eq!(report.templates_checked, 1);
}
