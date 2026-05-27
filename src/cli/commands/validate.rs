//! `crud-cli validate` command handler.

use crate::cli::args::{exit_with_envelope, ValidateArgs};
use crate::cli::output::emit_success;
use crate::core::validator::{self, ValidateParams};

/// Runs template validation and maps the result to a process exit code.
pub fn run_validate(args: ValidateArgs) -> i32 {
    let params = ValidateParams {
        type_filter: args.type_.map(|s| {
            s.split(',')
                .map(str::trim)
                .filter(|p| !p.is_empty())
                .map(str::to_string)
                .collect()
        }),
    };

    match validator::run(params) {
        Ok(report) => {
            emit_success(Some(&format!(
                "validate ok: {} templates",
                report.templates_checked
            )));
            0
        }
        Err(envelope) => exit_with_envelope(&envelope),
    }
}
