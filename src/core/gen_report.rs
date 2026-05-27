//! Generation outcome report (`Serialize` for future `--json`).

use serde::Serialize;
use std::path::PathBuf;

/// Files written, skipped (dry-run), or conflicting during `gen`.
#[derive(Debug, Default, Clone, Serialize)]
pub struct GenReport {
    /// Paths successfully written.
    pub written: Vec<PathBuf>,
    /// Paths that would be written but were skipped (e.g. dry-run).
    pub skipped: Vec<PathBuf>,
    /// Reserved for conflict tracking (v1 aborts via `FileConflict` before populating).
    pub conflicts: Vec<PathBuf>,
}
