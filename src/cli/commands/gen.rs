//! `crud-cli gen` command handler.

use crate::cli::args::{exit_with_envelope, GenArgs};
use crate::cli::output::emit_success;
use crate::core::error::ErrorEnvelope;
use crate::core::gen_pipeline;
use crate::core::gen_run::GenRunParams;

/// Runs `gen` end-to-end: validate args, pipeline, success line.
pub fn run_gen(args: GenArgs) -> i32 {
    if let Err(envelope) = args.validate_inputs() {
        return exit_with_envelope(&envelope);
    }

    let params = match gen_run_params_from_args(args) {
        Ok(p) => p,
        Err(envelope) => return exit_with_envelope(&envelope),
    };

    match gen_pipeline::run(params) {
        Ok(report) => {
            let line = format!("生成 {} 个文件", report.written.len());
            emit_success(Some(&line));
            0
        }
        Err(envelope) => exit_with_envelope(&envelope),
    }
}

fn gen_run_params_from_args(args: GenArgs) -> Result<GenRunParams, ErrorEnvelope> {
    let name = args
        .name
        .ok_or_else(|| missing_gen_flag("name", "missing_name"))?;
    let fields_src = args
        .fields
        .ok_or_else(|| missing_gen_flag("fields", "missing_fields"))?;
    let package = args
        .package
        .ok_or_else(|| missing_gen_flag("package", "missing_package"))?;
    let table = args
        .table
        .ok_or_else(|| missing_gen_flag("table", "missing_table"))?;
    let type_filter = args.type_.map(|s| {
        s.split(',')
            .map(str::trim)
            .filter(|p| !p.is_empty())
            .map(str::to_string)
            .collect()
    });
    Ok(GenRunParams {
        name,
        fields_src,
        package,
        table,
        file: args.file,
        type_filter,
        dry_run: args.dry_run,
        force: args.force,
    })
}

fn missing_gen_flag(flag: &'static str, reason: &'static str) -> ErrorEnvelope {
    let mut details = serde_json::Map::new();
    details.insert("flag".into(), serde_json::Value::String(flag.to_string()));
    ErrorEnvelope::user_error_with_reason(
        format!("missing required --{flag}"),
        reason,
        details,
        format!("provide --{flag} for DSL generation"),
    )
}
