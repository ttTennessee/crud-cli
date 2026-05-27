//! Two-phase filesystem writer: plan then atomic commit (D-14, CONF-06/07).

use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use super::config::OverwritePolicy;
use super::error::{ErrorEnvelope, Kind};

/// Single destination write intent.
#[derive(Debug, Clone)]
pub struct WriteTarget {
    /// Final path on disk.
    pub path: PathBuf,
    /// Bytes to persist at commit time.
    pub content: Vec<u8>,
}

/// Overwrite gate applied during preflight (CONF-08).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OverwriteContext {
    /// Policy from setup configuration.
    pub policy: OverwritePolicy,
    /// CLI `--force` flag.
    pub force: bool,
}

/// Planned batch validated for commit.
#[derive(Debug, Clone)]
pub struct WritePlan {
    targets: Vec<WriteTarget>,
}

/// Preflight all targets; abort entire batch on any denied conflict (D-14).
pub fn plan(
    targets: &[WriteTarget],
    overwrite: OverwriteContext,
) -> Result<WritePlan, ErrorEnvelope> {
    for target in targets {
        if target.path.exists() && !allows_overwrite(overwrite, target.path.as_path())? {
            return Err(ErrorEnvelope::file_conflict(
                format!("file exists: {}", target.path.display()),
                &target.path,
            ));
        }
    }
    Ok(WritePlan {
        targets: targets.to_vec(),
    })
}

fn allows_overwrite(ctx: OverwriteContext, _path: &Path) -> Result<bool, ErrorEnvelope> {
    match ctx.policy {
        OverwritePolicy::Never => Ok(false),
        OverwritePolicy::ForceOnly => Ok(ctx.force),
        OverwritePolicy::Always => Ok(true),
    }
}

/// Commit all planned writes via tempfile + fsync + atomic rename (CONF-07).
pub fn commit(plan: WritePlan) -> Result<(), ErrorEnvelope> {
    for target in plan.targets {
        atomic_write(&target.path, &target.content)?;
    }
    Ok(())
}

fn atomic_write(path: &Path, content: &[u8]) -> Result<(), ErrorEnvelope> {
    let parent = path.parent().ok_or_else(|| write_error(path, "missing parent directory"))?;
    fs::create_dir_all(parent).map_err(|e| write_error(path, format!("create dirs: {e}")))?;

    let mut temp = tempfile::NamedTempFile::new_in(parent)
        .map_err(|e| write_error(path, format!("tempfile: {e}")))?;
    temp.write_all(content)
        .map_err(|e| write_error(path, format!("write temp: {e}")))?;
    temp.as_file()
        .sync_all()
        .map_err(|e| write_error(path, format!("fsync temp: {e}")))?;
    temp.persist(path)
        .map_err(|e| write_error(path, format!("rename: {}", e.error)))?;

    sync_parent_dir(parent, path);
    Ok(())
}

fn sync_parent_dir(parent: &Path, path: &Path) {
    if let Ok(dir) = File::open(parent) {
        let _ = dir.sync_all();
    }
    let _ = OpenOptions::new().write(true).open(path).and_then(|f| f.sync_all());
}

fn write_error(path: &Path, msg: impl Into<String>) -> ErrorEnvelope {
    ErrorEnvelope {
        kind: Kind::ConfigError,
        msg: msg.into(),
        exit_code: Kind::ConfigError.exit_code(),
        hint: String::new(),
        details: {
            let mut m = serde_json::Map::new();
            m.insert(
                "path".into(),
                serde_json::Value::String(path.display().to_string()),
            );
            m
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn overwrite_gate_never_denies() {
        assert!(matches!(
            allows_overwrite(
                OverwriteContext {
                    policy: OverwritePolicy::Never,
                    force: false,
                },
                Path::new("x")
            ),
            Ok(false)
        ));
    }
}
