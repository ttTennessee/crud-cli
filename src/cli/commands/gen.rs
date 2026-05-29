//! `crud-cli gen` command handler.

use std::collections::BTreeMap;

use crate::cli::args::{exit_with_envelope, GenArgs};
use crate::cli::output::{
    emit_condition_skips, emit_dry_run_listing, emit_stdout_render, emit_success,
};
use crate::core::error::ErrorEnvelope;
use crate::core::i18n::{self, keys};
use crate::core::gen_pipeline;
use crate::core::gen_run::GenRunParams;
use crate::core::template_variables::parse_var_arg;

/// Runs `gen` end-to-end: validate args, pipeline, success line.
pub fn run_gen(args: GenArgs) -> i32 {
    if let Err(envelope) = args.validate_inputs() {
        return exit_with_envelope(&envelope);
    }

    let dry_run = args.dry_run;
    let to_stdout = args.stdout;
    let params = match gen_run_params_from_args(args) {
        Ok(p) => p,
        Err(envelope) => return exit_with_envelope(&envelope),
    };

    match gen_pipeline::run(params) {
        Ok(report) => {
            if to_stdout {
                // Preview mode: stdout carries only the rendered content so it
                // can be captured/piped verbatim (no success line, no skip
                // notices to avoid corrupting it).
                emit_stdout_render(&report.rendered);
                return 0;
            }
            emit_condition_skips(&report.skipped_by_condition);
            if dry_run {
                emit_dry_run_listing(&report.dry_run_lines);
                let line = i18n::tf(
                    keys::GEN_SUCCESS_DRY_RUN,
                    &[
                        ("count", &report.skipped.len().to_string()),
                        ("conflicts", &report.conflicts.len().to_string()),
                    ],
                );
                emit_success(Some(&line));
            } else {
                let line = i18n::tf(
                    keys::GEN_SUCCESS_WRITTEN,
                    &[("count", &report.written.len().to_string())],
                );
                emit_success(Some(&line));
            }
            0
        }
        Err(envelope) => exit_with_envelope(&envelope),
    }
}

fn gen_run_params_from_args(args: GenArgs) -> Result<GenRunParams, ErrorEnvelope> {
    let type_filter = args.type_.map(|s| {
        s.split(',')
            .map(str::trim)
            .filter(|p| !p.is_empty())
            .map(str::to_string)
            .collect()
    });

    let mut cli_vars = BTreeMap::new();
    for raw in &args.var {
        let (k, v) = parse_var_arg(raw)?;
        cli_vars.insert(k, v);
    }

    if let Some(ref path) = args.file {
        return Ok(GenRunParams {
            name: args.name,
            fields_src: None,
            package: args.package,
            table: args.table,
            file: Some(path.clone()),
            type_filter,
            dry_run: args.dry_run,
            stdout: args.stdout,
            force: args.force,
            output_dir: args.output,
            cli_vars,
        });
    }

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
    Ok(GenRunParams {
        name: Some(name),
        fields_src: Some(fields_src),
        package: Some(package),
        table: Some(table),
        file: None,
        type_filter,
        dry_run: args.dry_run,
        stdout: args.stdout,
        force: args.force,
        output_dir: args.output,
        cli_vars,
    })
}

fn missing_gen_flag(flag: &'static str, reason: &'static str) -> ErrorEnvelope {
    let mut details = serde_json::Map::new();
    details.insert("flag".into(), serde_json::Value::String(flag.to_string()));
    ErrorEnvelope::user_error_with_reason(
        format!("missing required --{flag}"),
        reason,
        details,
        i18n::tf(keys::ERROR_GEN_MISSING_FLAG, &[("flag", flag)]),
    )
}
