//! `crud-cli gen` command handler.

use crate::cli::args::{exit_with_envelope, GenArgs};
use crate::cli::output::emit_success;
use crate::core::gen_pipeline;

/// Runs `gen` end-to-end: validate args, pipeline, success line.
pub fn run_gen(args: GenArgs) -> i32 {
    if let Err(envelope) = args.validate_inputs() {
        return exit_with_envelope(&envelope);
    }

    match gen_pipeline::run(args) {
        Ok(report) => {
            let line = format!("生成 {} 个文件", report.written.len());
            emit_success(Some(&line));
            0
        }
        Err(envelope) => exit_with_envelope(&envelope),
    }
}
