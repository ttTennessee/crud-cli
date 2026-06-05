//! MCP resource URIs and read handlers (`crud://…`).
//!
//! Per-template `variables` / `field_types` schemas are intentionally **not**
//! exposed as resources: the `crud_describe_templates` tool already returns them
//! (alongside paths and project info), so duplicating them here would drift.

use std::path::Path;

use crate::core::field_dsl::RESERVED_VARIABLE_NAMES;
use serde_json::json;

/// Resource URI for entity.json input documentation.
pub const URI_ENTITY_GUIDE: &str = "crud://schema/entity_guide";

/// Resource URI for entity.json examples aggregated from `_example*.json` files.
pub const URI_ENTITY_EXAMPLE: &str = "crud://schema/entity_example";

/// Resource URI for built-in / reserved template variable names.
pub const URI_BUILTINS: &str = "crud://schema/builtins";

const MIME_JSON: &str = "application/json";
const MIME_MARKDOWN: &str = "text/markdown";

/// Static resource descriptor: `(uri, name, description, mime_type)`.
pub type ResourceDescriptor = (&'static str, &'static str, &'static str, &'static str);

/// All resource descriptors for `list_resources`.
pub fn list_resources(templates_root: &Path) -> Vec<ResourceDescriptor> {
    let mut list = vec![
        (
            URI_ENTITY_GUIDE,
            "entity_guide",
            "entity.json input specification for code generation",
            MIME_MARKDOWN,
        ),
        (
            URI_BUILTINS,
            "builtins",
            "Built-in template context names and reserved identifiers",
            MIME_JSON,
        ),
    ];
    if has_example_files(templates_root) {
        list.push((
            URI_ENTITY_EXAMPLE,
            "entity_example",
            "entity.json examples from the active template bundle (_example*.json)",
            MIME_JSON,
        ));
    }
    list
}

/**
 * Reads a resource for `uri`, returning `(body, mime_type)`.
 */
pub fn read_resource(
    uri: &str,
    templates_root: &Path,
) -> Result<(String, &'static str), String> {
    match uri {
        URI_ENTITY_GUIDE => Ok((
            include_str!("../../agent-resources/entity-json-guide.md").to_string(),
            MIME_MARKDOWN,
        )),
        URI_ENTITY_EXAMPLE => Ok((load_examples_json(templates_root)?, MIME_JSON)),
        URI_BUILTINS => Ok((builtins_json(), MIME_JSON)),
        _ => Err(format!("unknown resource uri: {uri}")),
    }
}

/// Returns true when at least one `_example*.json` exists under `templates_root`.
fn has_example_files(templates_root: &Path) -> bool {
    let Ok(entries) = std::fs::read_dir(templates_root) else {
        return false;
    };
    entries.flatten().any(|e| is_example_entry(&e))
}

/// Reads all `_example*.json` files, parses each as a JSON value, and returns
/// a pretty-printed JSON array combining them all.
fn load_examples_json(templates_root: &Path) -> Result<String, String> {
    let mut entries: Vec<std::fs::DirEntry> = std::fs::read_dir(templates_root)
        .map_err(|e| format!("read templates dir: {e}"))?
        .flatten()
        .filter(|e| is_example_entry(e))
        .collect();
    entries.sort_by_key(|e| e.file_name());

    let mut examples = Vec::new();
    for entry in entries {
        let path = entry.path();
        let raw = std::fs::read_to_string(&path)
            .map_err(|e| format!("read {}: {e}", path.display()))?;
        let value: serde_json::Value = serde_json::from_str(&raw)
            .map_err(|e| format!("parse {}: {e}", path.display()))?;
        examples.push(value);
    }

    serde_json::to_string_pretty(&examples).map_err(|e| format!("serialize examples: {e}"))
}

fn is_example_entry(entry: &std::fs::DirEntry) -> bool {
    let name = entry.file_name();
    let s = name.to_string_lossy();
    s.starts_with("_example") && s.ends_with(".json")
}

fn builtins_json() -> String {
    serde_json::to_string_pretty(&json!({
        "reserved_variable_names": RESERVED_VARIABLE_NAMES,
        "note": "Do not declare these in _variables.toml or JSON variables; they are injected by crud-cli at render time.",
        "field_name_reserved": ["model", "table", "table_comment", "package", "package_path", "fields"],
    }))
    .unwrap_or_else(|_| "{}".into())
}
