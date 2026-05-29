//! Generation outcome report (`Serialize` for future `--json`).

use serde::Serialize;
use std::path::PathBuf;

/// One resolved output line in `--dry-run` mode .
#[derive(Debug, Clone, Serialize)]
pub struct DryRunLine {
    pub path: PathBuf,
    pub line_count: usize,
    pub conflict: bool,
}

/// Files written, skipped (dry-run), or conflicting during `gen`.
#[derive(Debug, Default, Clone, Serialize)]
pub struct GenReport {
    /// Paths successfully written.
    pub written: Vec<PathBuf>,
    /// Paths that would be written but were skipped (e.g. dry-run).
    pub skipped: Vec<PathBuf>,
    /// Paths blocked by overwrite policy under dry-run.
    pub conflicts: Vec<PathBuf>,
    /// Populated only when `dry_run` is true.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub dry_run_lines: Vec<DryRunLine>,
}
