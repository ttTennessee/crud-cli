//! Maps core types to JSON payloads for MCP tool/resource responses.

use serde_json::{json, Value};

use crate::core::error::ErrorEnvelope;
use crate::core::gen_report::GenReport;

/**
 * Serializes an [`ErrorEnvelope`] for MCP tool output.
 */
pub fn envelope_to_value(envelope: &ErrorEnvelope) -> Value {
    json!({
        "ok": false,
        "kind": envelope.kind,
        "msg": envelope.msg,
        "exit_code": envelope.exit_code,
        "hint": envelope.hint,
        "details": envelope.details,
    })
}

/**
 * Maps a generate [`GenReport`] to JSON (`written` paths).
 */
pub fn generate_report_value(report: &GenReport) -> Value {
    json!({
        "ok": true,
        "written": report
            .written
            .iter()
            .map(|p| p.display().to_string())
            .collect::<Vec<_>>(),
        "skipped_by_condition": report
            .skipped_by_condition
            .iter()
            .map(|p| p.display().to_string())
            .collect::<Vec<_>>(),
    })
}
