//! Generation run parameters (CLI-agnostic; `GenArgs` maps here in `cli`).

use std::path::PathBuf;

/// Inputs for `gen_pipeline::run` without a `clap` dependency (FOUND-02).
#[derive(Debug, Clone)]
pub struct GenRunParams {
    pub name: Option<String>,
    pub fields_src: Option<String>,
    pub package: Option<String>,
    pub table: Option<String>,
    pub file: Option<PathBuf>,
    pub type_filter: Option<Vec<String>>,
    pub dry_run: bool,
    pub force: bool,
    pub output_dir: Option<PathBuf>,
}
