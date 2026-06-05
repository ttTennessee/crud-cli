//! Entity-schema reference content for the `crud_entity_schema` tool.
//!
//! Per-template `variables` / `field_types` schemas are intentionally **not**
//! exposed here: the `crud_describe_templates` tool already returns them
//! (alongside paths and project info), so duplicating them would drift.

use std::path::Path;

use crate::core::field_dsl::RESERVED_VARIABLE_NAMES;
use serde_json::json;

/// Reads an entity-schema reference body by short name.
///
/// `name` is one of `guide` | `example` | `builtins`.
pub fn read_entity_schema(name: &str, templates_root: &Path) -> Result<String, String> {
    match name {
        "guide" => Ok(include_str!("../../agent-resources/entity-json-guide.md").to_string()),
        "example" => load_examples_json(templates_root),
        "builtins" => Ok(builtins_json()),
        _ => Err(format!(
            "unknown entity schema name: {name} (expected: guide | example | builtins)"
        )),
    }
}

/// Reads all `_example*.json` files, parses each as a JSON value, and returns
/// a pretty-printed JSON array combining them all.
fn load_examples_json(templates_root: &Path) -> Result<String, String> {
    let mut entries: Vec<std::fs::DirEntry> = std::fs::read_dir(templates_root)
        .map_err(|e| format!("read templates dir: {e}"))?
        .flatten()
        .filter(is_example_entry)
        .collect();
    if entries.is_empty() {
        return Err(format!(
            "no _example*.json files under {} — this template bundle does not ship examples",
            templates_root.display()
        ));
    }
    entries.sort_by_key(|e| e.file_name());

    let mut examples = Vec::new();
    for entry in entries {
        let path = entry.path();
        let raw =
            std::fs::read_to_string(&path).map_err(|e| format!("read {}: {e}", path.display()))?;
        let value: serde_json::Value =
            serde_json::from_str(&raw).map_err(|e| format!("parse {}: {e}", path.display()))?;
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
