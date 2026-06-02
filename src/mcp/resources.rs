//! MCP resource URIs and read handlers (`crud://…`).

use crate::core::field_dsl::RESERVED_VARIABLE_NAMES;
use serde_json::json;

/// Resource URI for `_variables.toml`.
pub const URI_VARIABLES: &str = "crud://templates/variables";

/// Resource URI for `_field_types.toml`.
pub const URI_FIELD_TYPES: &str = "crud://templates/field-types";

/// Resource URI for entity.json input documentation.
pub const URI_ENTITY_SCHEMA: &str = "crud://schema/entity";

/// Resource URI for built-in / reserved template variable names.
pub const URI_BUILTINS: &str = "crud://builtins";

/// Resource URI for template authoring guide (markdown).
pub const URI_TEMPLATE_AUTHORING: &str = "crud://docs/template-authoring";

/// All static resource descriptors for `list_resources`.
pub fn list_static_resources() -> Vec<(&'static str, &'static str)> {
    vec![
        (URI_VARIABLES, "Template per-call variables schema (_variables.toml)"),
        (URI_FIELD_TYPES, "Allowed field types schema (_field_types.toml)"),
        (URI_ENTITY_SCHEMA, "entity.json input specification for code generation"),
        (URI_BUILTINS, "Built-in template context names and reserved identifiers"),
        (URI_TEMPLATE_AUTHORING, "Guide for writing crud-cli Handlebars templates"),
    ]
}

/**
 * Reads resource body text for `uri`, using `templates_root` when needed.
 */
pub fn read_resource(
    uri: &str,
    templates_root: &std::path::Path,
) -> Result<String, String> {
    match uri {
        URI_VARIABLES => super::context::read_bundle_file(templates_root, "_variables.toml")
            .map_err(|e| e.msg),
        URI_FIELD_TYPES => super::context::read_bundle_file(templates_root, "_field_types.toml")
            .map_err(|e| e.msg),
        URI_ENTITY_SCHEMA => Ok(include_str!("../../docs/zh-CN/json-entity-input.md").to_string()),
        URI_BUILTINS => Ok(builtins_json()),
        URI_TEMPLATE_AUTHORING => {
            Ok(include_str!("../../docs/zh-CN/template-authoring.md").to_string())
        }
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
