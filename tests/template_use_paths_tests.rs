//! Template [paths] propagation: manifest parsing + cmd_use application.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use crud_cli::core::config::{Backend, Frontend, SetupConfig, SetupSelections};
use crud_cli::core::template_meta_global::{TemplateManifest, MANIFEST_FILENAME};
use std::fs;
use std::sync::{Mutex, OnceLock};
use tempfile::TempDir;

static HOME_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
static CWD_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

struct EnvVarGuard {
    key: &'static str,
    prev: Option<String>,
}

impl EnvVarGuard {
    /// `crud-cli` honors `CRUD_HOME` as an explicit override for the global
    /// home (`~/.crud`). We use it instead of `$HOME` because Windows resolves
    /// the user home via `SHGetKnownFolderPath` and ignores env vars entirely.
    fn set(key: &'static str, value: &std::path::Path) -> Self {
        let prev = std::env::var(key).ok();
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

// ── Cycle 1: manifest parsing ──────────────────────────────────────────────

#[test]
fn manifest_with_paths_aux_deserializes() {
    let toml = r#"
backend  = "java"
frontend = "vue"

[paths.lang]
java = "src/main/java"

[paths.aux]
resources  = "src/main/resources"
liquibase  = "src/main/resources/db/changelog"
"#;
    let manifest: TemplateManifest = toml::from_str(toml).expect("parse");
    let paths = manifest.paths.expect("paths should be Some");
    assert_eq!(
        paths.lang.get("java").map(String::as_str),
        Some("src/main/java")
    );
    assert_eq!(
        paths.aux.get("liquibase").map(String::as_str),
        Some("src/main/resources/db/changelog")
    );
    assert_eq!(
        paths.aux.get("resources").map(String::as_str),
        Some("src/main/resources")
    );
}

#[test]
fn manifest_without_paths_has_none() {
    let toml = "backend = \"java\"\nfrontend = \"vue\"\n";
    let manifest: TemplateManifest = toml::from_str(toml).expect("parse");
    assert!(manifest.paths.is_none());
}

// ── Cycle 2: cmd_use applies template paths ────────────────────────────────

fn install_template_with_paths(home: &std::path::Path, name: &str, version: &str) {
    let bundle = home
        .join(".crud")
        .join("templates")
        .join(name)
        .join(version);
    fs::create_dir_all(&bundle).unwrap();
    let manifest_toml = r#"backend = "java"
frontend = "vue"

[paths.lang]
java = "src/main/java"

[paths.aux]
resources = "src/main/resources"
liquibase = "src/main/resources/db/changelog"
"#;
    fs::write(bundle.join(MANIFEST_FILENAME), manifest_toml).unwrap();
}

fn install_template_without_paths(home: &std::path::Path, name: &str, version: &str) {
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
}

fn run_template_use(project: &std::path::Path, home: &std::path::Path, template_ref: &str) -> i32 {
    let _home_lock = HOME_LOCK.get_or_init(|| Mutex::new(())).lock().unwrap();
    let _cwd_lock = CWD_LOCK.get_or_init(|| Mutex::new(())).lock().unwrap();
    let _home = EnvVarGuard::set("CRUD_HOME", home);
    let prev = std::env::current_dir().unwrap();
    std::env::set_current_dir(project).unwrap();
    let args = crud_cli::cli::args::TemplateArgs {
        command: crud_cli::cli::args::TemplateCommand::Use {
            name: template_ref.to_string(),
            yes: true,
        },
    };
    let code = crud_cli::cli::commands::template::run_template(args);
    std::env::set_current_dir(prev).unwrap();
    code
}

#[test]
fn cmd_use_replaces_project_paths_from_template() {
    let home_dir = TempDir::new().unwrap();
    let project_dir = TempDir::new().unwrap();
    install_template_with_paths(home_dir.path(), "eladmin", "1.0.0");
    // Project starts with Go paths (different from template)
    {
        let crud = project_dir.path().join(".crud");
        fs::create_dir_all(&crud).unwrap();
        let mut cfg = SetupConfig::from_selections(SetupSelections {
            backend: Backend::Go,
            frontend: Frontend::None,
            template: None,
        });
        // Also put a custom aux that shouldn't survive the template switch
        cfg.paths.aux.insert("old-custom".into(), "old/path".into());
        fs::write(crud.join("setup.toml"), cfg.to_toml_pretty().unwrap()).unwrap();
    }

    let code = run_template_use(project_dir.path(), home_dir.path(), "eladmin@1.0.0");
    assert_eq!(code, 0, "template use should succeed");

    let setup_path = project_dir.path().join(".crud").join("setup.toml");
    let raw = fs::read_to_string(&setup_path).unwrap();
    let cfg: SetupConfig = toml::from_str(&raw).unwrap();

    assert_eq!(
        cfg.paths.lang.get("java").map(String::as_str),
        Some("src/main/java"),
        "lang.java should come from template"
    );
    assert_eq!(
        cfg.paths.aux.get("liquibase").map(String::as_str),
        Some("src/main/resources/db/changelog"),
        "aux.liquibase should come from template"
    );
    assert!(
        !cfg.paths.aux.contains_key("old-custom"),
        "old custom path should be replaced"
    );
}

#[test]
fn cmd_use_keeps_default_paths_when_template_has_none() {
    let home_dir = TempDir::new().unwrap();
    let project_dir = TempDir::new().unwrap();
    install_template_without_paths(home_dir.path(), "bare", "1.0.0");
    {
        let crud = project_dir.path().join(".crud");
        fs::create_dir_all(&crud).unwrap();
        let mut cfg = SetupConfig::from_selections(SetupSelections {
            backend: Backend::Java,
            frontend: Frontend::None,
            template: None,
        });
        cfg.paths.aux.insert("custom".into(), "custom/path".into());
        fs::write(crud.join("setup.toml"), cfg.to_toml_pretty().unwrap()).unwrap();
    }

    let code = run_template_use(project_dir.path(), home_dir.path(), "bare@1.0.0");
    assert_eq!(code, 0);

    let raw = fs::read_to_string(project_dir.path().join(".crud/setup.toml")).unwrap();
    let cfg: SetupConfig = toml::from_str(&raw).unwrap();
    // Paths should be untouched
    assert_eq!(
        cfg.paths.aux.get("custom").map(String::as_str),
        Some("custom/path"),
        "custom path should survive when template has no [paths]"
    );
}
