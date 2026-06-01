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

/// One rendered file carried back to the CLI under `--stdout` (preview mode):
/// nothing is written to disk; the CLI prints `content` to standard output.
#[derive(Debug, Clone, Serialize)]
pub struct RenderedFile {
    pub path: PathBuf,
    pub content: String,
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
    /// Output paths intentionally not generated because a template's
    /// `generateWhen`/`skipWhen` condition evaluated to skip. Distinct from
    /// `skipped` (dry-run) so agents can tell "deliberately omitted" apart from
    /// "would be written".
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub skipped_by_condition: Vec<PathBuf>,
    /// Populated only when `dry_run` is true.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub dry_run_lines: Vec<DryRunLine>,
    /// Populated only when `stdout` is true: rendered files the CLI prints to
    /// standard output instead of writing to disk.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub rendered: Vec<RenderedFile>,
}
