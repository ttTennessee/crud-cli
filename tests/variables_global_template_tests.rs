//! `_variables.toml` must load from the pinned global template bundle, not only
//! from project-local `.crud/templates/`.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use crud_cli::cli::args::GenArgs;
use crud_cli::cli::commands::gen::run_gen;
use crud_cli::core::config::{Backend, Frontend, SetupConfig, SetupSelections, TemplateRef};
use crud_cli::core::template_meta_global::MANIFEST_FILENAME;
use crud_cli::core::template_variables::SCHEMA_FILE_NAME;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use tempfile::TempDir;

static CWD_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
static HOME_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

struct EnvVarGuard {
    key: &'static str,
    prev: Option<String>,
}

impl EnvVarGuard {
    fn set(key: &'static str, value: &Path) -> Self {
        let prev = std::env::var(key).ok();
        // SAFETY: tests hold HOME_LOCK / CWD_LOCK so no concurrent env mutation.
        unsafe { std::env::set_var(key, value) };
        Self { key, prev }
    }
}

impl Drop for EnvVarGuard {
    fn drop(&mut self) {
        match &self.prev {
            Some(v) => unsafe { std::env::set_var(self.key, v) },
            None => unsafe { std::env::remove_var(self.key) },
        }
    }
}

fn install_global_bundle(home: &Path, name: &str, version: &str) -> PathBuf {
    let bundle = home
        .join(".crud")
        .join("templates")
        .join(name)
        .join(version);
    fs::create_dir_all(&bundle).unwrap();
    fs::write(
        bundle.join(MANIFEST_FILENAME),
        "backend = \"java\"\nfrontend = \"vue\"\n",
    )
    .unwrap();
    fs::write(
        bundle.join(SCHEMA_FILE_NAME),
        "[has_import]\ndescription = \"toggle\"\ntype = \"bool\"\ndefault = false\n",
    )
    .unwrap();
    fs::write(bundle.join("Out.txt.hbs"), "import={{has_import}}\n").unwrap();
    bundle
}

fn seed_project_with_global_template(project: &Path, template: &str) {
    let crud = project.join(".crud");
    fs::create_dir_all(&crud).unwrap();
    let cfg = SetupConfig::from_selections(SetupSelections {
        backend: Backend::Java,
        frontend: Frontend::Vue,
        template: Some(TemplateRef::parse(template).expect("template ref")),
    });
    fs::write(crud.join("setup.toml"), cfg.to_toml_pretty().unwrap()).unwrap();
    assert!(
        !crud.join("templates").join(SCHEMA_FILE_NAME).exists(),
        "test assumes no project-local schema"
    );
}

fn run_gen_in(project: &Path, home: &Path, args: GenArgs) -> i32 {
    let _cwd_lock = CWD_LOCK.get_or_init(|| Mutex::new(())).lock().unwrap();
    let _home_lock = HOME_LOCK.get_or_init(|| Mutex::new(())).lock().unwrap();
    let _home = EnvVarGuard::set("HOME", home);
    let prev = std::env::current_dir().unwrap();
    std::env::set_current_dir(project).unwrap();
    let code = run_gen(args);
    std::env::set_current_dir(prev).unwrap();
    code
}

#[test]
fn global_template_variables_schema_used_without_project_local_schema() {
    let home_dir = TempDir::new().unwrap();
    let project_dir = TempDir::new().unwrap();
    install_global_bundle(home_dir.path(), "eladmin", "1.0.0");
    seed_project_with_global_template(project_dir.path(), "eladmin@1.0.0");

    let args = GenArgs {
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
    };
    assert_eq!(
        run_gen_in(project_dir.path(), home_dir.path(), args),
        0,
        "gen should accept schema from ~/.crud/templates/<name>/<version>/"
    );
    let out = fs::read_to_string(project_dir.path().join("Out.txt")).unwrap();
    assert!(out.contains("import=false"), "schema default should apply: {out}");
}
