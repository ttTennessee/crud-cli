//! MCP resource URIs and read handlers (`crud://…`).
//!
//! Per-template `variables` / `field_types` schemas are intentionally **not**
//! exposed as resources: the `crud_describe_templates` tool already returns them
//! (alongside paths and project info), so duplicating them here would drift.

use crate::core::field_dsl::RESERVED_VARIABLE_NAMES;
use serde_json::json;

/// Resource URI for entity.json input documentation.
pub const URI_ENTITY_SCHEMA: &str = "crud://schema/entity";

/// Resource URI for built-in / reserved template variable names.
pub const URI_BUILTINS: &str = "crud://schema/builtins";

const MIME_JSON: &str = "application/json";
const MIME_MARKDOWN: &str = "text/markdown";

/// Static resource descriptor: `(uri, name, mime_type)`.
pub type ResourceDescriptor = (&'static str, &'static str, &'static str);

/// All static resource descriptors for `list_resources`.
pub fn list_static_resources() -> Vec<ResourceDescriptor> {
    vec![
        (
            URI_ENTITY_SCHEMA,
            "entity.json input specification for code generation",
            MIME_MARKDOWN,
        ),
        (
            URI_BUILTINS,
            "Built-in template context names and reserved identifiers",
            MIME_JSON,
        ),
    ]
}

/**
 * Reads a resource for `uri`, returning `(body, mime_type)`.
 */
pub fn read_resource(
    uri: &str,
    _templates_root: &std::path::Path,
) -> Result<(String, &'static str), String> {
    match uri {
        URI_ENTITY_SCHEMA => Ok((
            include_str!("../../agent-resources/json-entity-input.md").to_string(),
            MIME_MARKDOWN,
        )),
        URI_BUILTINS => Ok((builtins_json(), MIME_JSON)),
        _ => Err(format!("unknown resource uri: {uri}")),
    }
}

fn builtins_json() -> String {
    serde_json::to_string_pretty(&json!({
        "reserved_variable_names": RESERVED_VARIABLE_NAMES,
        "note": "Do not declare these in _variables.toml or JSON variables; they are injected by crud-cli at render time.",
        "field_name_reserved": ["model", "table", "table_comment", "package", "package_path", "fields"],
    }))
    .unwrap_or_else(|_| "{}".into())
}
