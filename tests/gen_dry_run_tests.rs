//! `--dry-run` listing and zero writes (GEN-08).
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use crud_cli::cli::args::GenArgs;
use crud_cli::cli::commands::gen::run_gen;
use std::fs;
use std::io::Write;
use std::sync::{Mutex, OnceLock};
use tempfile::TempDir;

static CWD_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

fn cwd_guard() -> std::sync::MutexGuard<'static, ()> {
    CWD_LOCK.get_or_init(|| Mutex::new(())).lock().unwrap()
}

fn seed(root: &std::path::Path) {
    let crud = root.join(".crud");
    fs::create_dir_all(crud.join("templates")).unwrap();
    fs::write(crud.join("templates/a.txt.hbs"), "line1\nline2\n").unwrap();
    let mut setup = std::fs::File::create(crud.join("setup.toml")).unwrap();
    writeln!(
        setup,
        r#"
[project]
backend = "none"
frontend = "none"
component-library = "none"
[paths]
"#
    )
    .unwrap();
}

#[test]
fn dry_run_lists_paths_and_writes_nothing() {
    let dir = TempDir::new().unwrap();
    let root = dir.path();
    seed(root);
    fs::write(root.join("existing.txt"), "old").unwrap();

    let _lock = cwd_guard();
    let prev = std::env::current_dir().unwrap();
    std::env::set_current_dir(root).unwrap();

    // Pre-create output path that would conflict
    fs::write(root.join("a.txt"), "exists").unwrap();

    let code = run_gen(GenArgs {
        name: Some("X".into()),
        fields: Some("id:Long".into()),
        package: Some("com.x".into()),
        table: Some("t".into()),
        file: None,
        type_: None,
        dry_run: true,
        force: false,
        output: None,
        var: vec![],
    });
    std::env::set_current_dir(prev).unwrap();

    assert_eq!(code, 0);
    assert_eq!(fs::read_to_string(root.join("a.txt")).unwrap(), "exists");
}
