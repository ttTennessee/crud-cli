//! Phase 2 SC#5: batch conflict writes zero files.
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

#[test]
fn atomic_batch_conflict_writes_nothing() {
    let dir = TempDir::new().unwrap();
    let root = dir.path();
    let crud = root.join(".crud");
    fs::create_dir_all(crud.join("templates")).unwrap();
    fs::write(crud.join("templates/a.txt.hbs"), "A").unwrap();
    fs::write(crud.join("templates/b.txt.hbs"), "B-from-template").unwrap();
    fs::write(crud.join("templates/c.txt.hbs"), "C").unwrap();
    fs::write(root.join("b.txt"), "B-original").unwrap();

    let mut setup = std::fs::File::create(crud.join("setup.toml")).unwrap();
    writeln!(
        setup,
        r#"
[project]
backend = "none"
frontend = "none"
component-library = "none"
[paths]
[overwrite]
overwrite-policy = "never"
"#
    )
    .unwrap();

    let _lock = cwd_guard();
    let prev = std::env::current_dir().unwrap();
    std::env::set_current_dir(root).unwrap();
    let code = run_gen(GenArgs {
        name: Some("X".into()),
        fields: Some("id:Long".into()),
        package: Some("com.x".into()),
        table: Some("x".into()),
        file: None,
        type_: None,
        dry_run: false,
        force: false,
    });
    std::env::set_current_dir(prev).unwrap();

    assert_eq!(code, 3);
    assert!(!root.join("a.txt").exists());
    assert!(!root.join("c.txt").exists());
    assert_eq!(fs::read_to_string(root.join("b.txt")).unwrap(), "B-original");
}
