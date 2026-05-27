//! Task 1: fs_writer plan/commit transactional API (D-14, CONF-06/07).

use crud_cli::core::config::OverwritePolicy;
use crud_cli::core::error::Kind;
use crud_cli::core::fs_writer::{commit, plan, OverwriteContext, WriteTarget};
use std::fs;
use tempfile::TempDir;

fn ctx(policy: OverwritePolicy, force: bool) -> OverwriteContext {
    OverwriteContext { policy, force }
}

#[test]
fn fs_writer_conflict_aborts_batch() {
    let dir = TempDir::new().expect("tempdir");
    let root = dir.path();

    let existing = root.join("keep.txt");
    fs::write(&existing, b"original").expect("seed existing");

    let new_path = root.join("new.txt");
    let targets = vec![
        WriteTarget {
            path: existing.clone(),
            content: b"would overwrite".to_vec(),
        },
        WriteTarget {
            path: new_path.clone(),
            content: b"fresh".to_vec(),
        },
    ];

    let err = plan(&targets, ctx(OverwritePolicy::Never, false)).expect_err("conflict");
    assert_eq!(err.kind, Kind::FileConflict);
    assert_eq!(err.exit_code, 3);
    assert!(
        err.details.get("path").and_then(|v| v.as_str()).is_some(),
        "details.path required"
    );

    assert_eq!(fs::read(&existing).unwrap(), b"original");
    assert!(!new_path.exists(), "batch must not partially write");
}

#[test]
fn fs_writer_atomic_commit() {
    let dir = TempDir::new().expect("tempdir");
    let target = dir.path().join("nested").join("out.toml");

    let targets = vec![WriteTarget {
        path: target.clone(),
        content: b"[project]\nbackend = \"none\"\n".to_vec(),
    }];

    let write_plan = plan(&targets, ctx(OverwritePolicy::Always, false)).expect("plan ok");
    commit(write_plan).expect("commit ok");

    assert!(target.is_file());
    let content = fs::read_to_string(&target).expect("read committed");
    assert!(content.contains("[project]"));

    // Second commit overwrites atomically when policy allows.
    let targets2 = vec![WriteTarget {
        path: target.clone(),
        content: b"updated = true\n".to_vec(),
    }];
    let write_plan2 = plan(&targets2, ctx(OverwritePolicy::Always, false)).expect("plan2");
    commit(write_plan2).expect("commit2");
    assert_eq!(fs::read_to_string(&target).unwrap(), "updated = true\n");
}
