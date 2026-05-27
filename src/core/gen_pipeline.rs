//! `gen` orchestration: template discovery, render, and atomic write.

use crate::cli::args::GenArgs;
use crate::core::error::ErrorEnvelope;
use crate::core::gen_report::GenReport;

/// Runs the generation pipeline (Task 5 implements the full flow).
pub fn run(_args: GenArgs) -> Result<GenReport, ErrorEnvelope> {
    Err(ErrorEnvelope::user_error_with_reason(
        "generation pipeline not yet implemented",
        "not_implemented_until_task_5",
        serde_json::Map::new(),
        "complete Task 5 of plan 02-01",
    ))
}
