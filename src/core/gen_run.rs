//! Generation run parameters (CLI-agnostic; `GenArgs` maps here in `cli`).

use std::collections::BTreeMap;
use std::path::PathBuf;

use serde_json::Value;

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
    /// Parsed `--var key=value` entries. Declared in `_variables.toml`.
    pub cli_vars: BTreeMap<String, Value>,
}

impl Default for GenRunParams {
    fn default() -> Self {
        Self {
            name: None,
            fields_src: None,
            package: None,
            table: None,
            file: None,
            type_filter: None,
            dry_run: false,
            force: false,
            output_dir: None,
            cli_vars: BTreeMap::new(),
        }
    }
}
