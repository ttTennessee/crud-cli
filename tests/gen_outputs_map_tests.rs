//! `[templates.outputs]` path mapping (D-G28 layer 2).
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use crud_cli::cli::args::GenArgs;
use crud_cli::cli::commands::gen::run_gen;
use crud_cli::core::gen_context::build_context_from_input;
use crud_cli::core::gen_input::GenInput;
use crud_cli::core::field_dsl::Field;
use crud_cli::core::git_info::GitInfo;
use crud_cli::core::template_engine::render_template;
use std::fs;
use std::sync::{Mutex, OnceLock};
use tempfile::TempDir;

static CWD_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

fn cwd_guard() -> std::sync::MutexGuard<'static, ()> {
    CWD_LOCK.get_or_init(|| Mutex::new(())).lock().unwrap()
}

#[test]
fn outputs_map_handlebars_template_renders() {
    let input = GenInput {
        name: "User".into(),
        table: "u".into(),
        package: "com.x".into(),
        table_comment: String::new(),
        sub: None,
        fields: vec![Field {
            name: "id".into(),
            ty: "Long".into(),
            is_pk: false,
            nullable: false,
        }],
    };
    let setup = crud_cli::core::config::SetupConfig::from_selections(
        crud_cli::core::config::SetupSelections {
            backend: crud_cli::core::config::Backend::None,
            frontend: crud_cli::core::config::Frontend::None,
            template: None,
        },
    );
    let ctx = build_context_from_input(&input, &setup, &GitInfo::default(), &crud_cli::core::gen_context::UserIdentity::default()).unwrap();
    let out = render_template(
        "src/main/java/{{package_path}}/{{model_pascal}}.java",
        &ctx,
    )
    .unwrap();
    assert_eq!(out, "src/main/java/com/x/User.java");
}

#[test]
fn outputs_map_resolves_java_entity_path() {
    let dir = TempDir::new().unwrap();
    let root = dir.path();
    let crud = root.join(".crud");
    fs::create_dir_all(crud.join("templates/java")).unwrap();
    fs::write(
        crud.join("templates/java/Entity.java.hbs"),
        "class {{model_pascal}} {}",
    )
    .unwrap();

    const SETUP_TOML: &str = r#"
[project]
backend = "java"
frontend = "vue"


[paths.lang]
java = "src/main/java"
vue = "src/views"


[templates.outputs]
"java/Entity.java.hbs" = "src/main/java/{{package_path}}/{{model_pascal}}.java"
"#;
    fs::write(crud.join("setup.toml"), SETUP_TOML).unwrap();

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
    let out = root.join("src/main/java/com/x/User.java");
    let exists = out.is_file();
    std::env::set_current_dir(prev).unwrap();

    assert_eq!(code, 0, "run_gen exit code (tree: {:?})", list_files(root));
    assert!(
        exists,
        "expected {} (tree under {}: {:?})",
        out.display(),
        root.display(),
        list_files(root)
    );
}

fn list_files(root: &std::path::Path) -> Vec<String> {
    let mut out = Vec::new();
    if let Ok(walk) = std::fs::read_dir(root) {
        for entry in walk.flatten() {
            let p = entry.path();
            if p.is_file() {
                out.push(p.display().to_string());
            } else if p.is_dir() && !p.file_name().is_some_and(|n| n == ".crud") {
                for sub in list_files(&p) {
                    out.push(sub);
                }
            }
        }
    }
    out
}
