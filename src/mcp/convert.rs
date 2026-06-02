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
 * Serializes a successful validate result.
 */
pub fn validate_ok_value() -> Value {
    json!({ "ok": true })
}

/**
 * Maps a preview [`GenReport`] to JSON (`rendered` paths + contents).
 */
pub fn preview_report_value(report: &GenReport) -> Value {
    let rendered: Vec<Value> = report
        .rendered
        .iter()
        .map(|f| {
            json!({
                "path": f.path.display().to_string(),
                "content": f.content,
            })
        })
        .collect();
    json!({
        "ok": true,
        "rendered": rendered,
        "skipped_by_condition": report
            .skipped_by_condition
            .iter()
            .map(|p| p.display().to_string())
            .collect::<Vec<_>>(),
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
