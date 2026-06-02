//! Entity JSON validation without running the full generation pipeline.

use std::collections::BTreeMap;
use std::io::Write;
use std::path::Path;

use convert_case::{Case, Casing};
use serde_json::{json, Value};
use tempfile::NamedTempFile;

use crate::core::config::RuntimeConfig;
use crate::core::error::ErrorEnvelope;
use crate::core::field_dsl::Field;
use crate::core::field_types;
use crate::core::gen_input::{
    load_gen_input_with_specs_from_json, FieldSpec, GenCliOverrides, JsonLoadResult,
};
use crate::core::paths::{project_setup_toml, project_setup_user_toml};
use crate::core::template_loader;
use crate::core::template_variables;

use super::context::ProjectContext;

/**
 * Loads `entity_json`, normalizes field types, and validates variables against
 * the active template schemas (no file writes). Returns the normalized bundle.
 */
fn load_validated(
    ctx: &ProjectContext,
    entity_json: &str,
    cli_vars: &BTreeMap<String, Value>,
) -> Result<JsonLoadResult, ErrorEnvelope> {
    let mut tmp = NamedTempFile::new().map_err(|e| {
        ErrorEnvelope::user_error(
            format!("temp file: {e}"),
            None,
            None,
            "could not create temp file for entity JSON",
        )
    })?;
    tmp.write_all(entity_json.as_bytes()).map_err(|e| {
        ErrorEnvelope::user_error(
            format!("write temp: {e}"),
            None,
            None,
            "could not write entity JSON",
        )
    })?;
    let path = tmp.path().to_path_buf();

    let mut loaded = load_gen_input_with_specs_from_json(&path, GenCliOverrides::default())?;

    let field_type_schema = field_types::load_schema(&ctx.templates_root)?;
    field_types::normalize_and_validate(&field_type_schema, &mut loaded.input.fields)?;
    if let Some(sub) = loaded.input.sub.as_mut() {
        field_types::normalize_and_validate(&field_type_schema, &mut sub.fields)?;
    }

    let schema = template_variables::load_schema(&ctx.templates_root)?;
    let _merged = template_variables::merge_values(&schema, cli_vars, &loaded.variables)?;

    Ok(loaded)
}

/**
 * Validates `entity_json` against the active template schemas (no file writes).
 */
pub fn validate_entity_json(
    ctx: &ProjectContext,
    entity_json: &str,
    cli_vars: &BTreeMap<String, Value>,
) -> Result<(), ErrorEnvelope> {
    load_validated(ctx, entity_json, cli_vars).map(|_| ())
}

/// Markdown table header (+ separator) shared by master and sub sections.
fn table_header() -> &'static str {
    "| 字段名 | 列名 | 类型 | 主键 | 必填 | 长度 | 注释 | 标记 |\n|---|---|---|---|---|---|---|---|\n"
}

/// `required` reflects the form-level flag when present, else DB nullability.
fn required_flag(spec: &FieldSpec) -> bool {
    if let Some(Value::Bool(b)) = spec.extra.get("required") {
        return *b;
    }
    !spec.nullable
}

/// Collects boolean-true `extra` keys as display tags (excluding `required`).
fn collect_tags(spec: &FieldSpec) -> Vec<String> {
    let mut tags: Vec<String> = spec
        .extra
        .iter()
        .filter(|(k, v)| k.as_str() != "required" && matches!(v, Value::Bool(true)))
        .map(|(k, _)| k.clone())
        .collect();
    tags.sort();
    tags
}

/// Length from the typed `length` field, falling back to `extra.length`.
fn length_value(spec: &FieldSpec) -> Value {
    spec.length
        .map(Value::from)
        .or_else(|| spec.extra.get("length").cloned())
        .unwrap_or(Value::Null)
}

/// Default from the typed `default` field, falling back to `extra.default`.
fn default_value(spec: &FieldSpec) -> Value {
    spec.default
        .clone()
        .or_else(|| spec.extra.get("default").cloned())
        .unwrap_or(Value::Null)
}

/// Renders a scalar [`Value`] for a markdown cell (empty for null).
fn cell(value: &Value) -> String {
    match value {
        Value::Null => String::new(),
        Value::String(s) => s.replace('|', "\\|"),
        other => other.to_string(),
    }
}

/**
 * Builds both the machine-readable field rows and the markdown table body,
 * pairing normalized types (`norms`) with rich input attributes (`specs`).
 */
fn build_field_section(norms: &[Field], specs: &[FieldSpec]) -> (Vec<Value>, String) {
    let mut rows = Vec::with_capacity(specs.len());
    let mut md = String::new();
    for (norm, spec) in norms.iter().zip(specs.iter()) {
        let column = spec.name.to_case(Case::Snake);
        let length = length_value(spec);
        let required = required_flag(spec);
        let tags = collect_tags(spec);
        rows.push(json!({
            "name": spec.name,
            "column": column,
            "type": norm.ty,
            "pk": norm.is_pk,
            "required": required,
            "length": length,
            "default": default_value(spec),
            "unique": spec.unique,
            "comment": spec.comment,
            "tags": tags,
        }));
        md.push_str(&format!(
            "| {} | {} | {} | {} | {} | {} | {} | {} |\n",
            cell(&Value::String(spec.name.clone())),
            column,
            norm.ty,
            if norm.is_pk { "✓" } else { "" },
            if required { "✓" } else { "" },
            cell(&length),
            cell(&Value::String(spec.comment.clone())),
            tags.join(", "),
        ));
    }
    (rows, md)
}

/**
 * Validates `entity_json` and returns its normalized structure as a table
 * (machine-readable `fields` plus a renderable `table_markdown`), without
 * rendering any template output.
 */
pub fn preview_entity_structure(
    ctx: &ProjectContext,
    entity_json: &str,
    cli_vars: &BTreeMap<String, Value>,
) -> Result<Value, ErrorEnvelope> {
    let loaded = load_validated(ctx, entity_json, cli_vars)?;
    let input = &loaded.input;

    let (field_rows, fields_md) = build_field_section(&input.fields, &loaded.field_specs);
    let pk = input.fields.iter().find(|f| f.is_pk).map(|f| f.name.clone());

    let mut md = format!("## {} ({})\n\n", input.name, input.table);
    if !input.table_comment.is_empty() {
        md.push_str(&format!("> {}\n\n", input.table_comment));
    }
    md.push_str("### 主表字段\n\n");
    md.push_str(table_header());
    md.push_str(&fields_md);

    let mut out = json!({
        "ok": true,
        "entity": {
            "name": input.name,
            "table": input.table,
            "table_comment": input.table_comment,
            "package": input.package,
            "pk": pk,
        },
        "fields": field_rows,
    });

    if let (Some(sub), Some(sub_specs)) = (input.sub.as_ref(), loaded.sub_field_specs.as_ref()) {
        let (sub_rows, sub_md) = build_field_section(&sub.fields, sub_specs);
        md.push_str(&format!(
            "\n### 子表 {} ({}) — fk: {}\n\n",
            sub.name, sub.table, sub.fk_field
        ));
        md.push_str(table_header());
        md.push_str(&sub_md);
        out["sub"] = json!({
            "name": sub.name,
            "table": sub.table,
            "table_comment": sub.table_comment,
            "fk_field": sub.fk_field,
            "fields": sub_rows,
        });
    }

    out["table_markdown"] = Value::String(md.clone());
    out["display_markdown"] = Value::String(md);
    out["must_display_to_user"] = Value::Bool(true);
    out["next_step"] = Value::String(
        "Show the table to user and wait for confirmation before generate.".into(),
    );
    out["prompt"] = Value::String(
        "Render `table_markdown` to the user to confirm field types, required flags, and \
         lengths. Do NOT echo the raw entity.json. To apply edits, modify the original \
         entity.json using `fields[].name` as the stable key, then call `preview` again or \
         `generate`."
            .into(),
    );
    Ok(out)
}

/**
 * Writes `entity_json` to a temp file and returns its path for `gen_pipeline::run`.
 */
pub fn entity_json_to_temp_path(entity_json: &str) -> Result<tempfile::TempPath, ErrorEnvelope> {
    let mut tmp = NamedTempFile::new().map_err(|e| {
        ErrorEnvelope::user_error(
            format!("temp file: {e}"),
            None,
            None,
            "could not create temp file for entity JSON",
        )
    })?;
    tmp.write_all(entity_json.as_bytes()).map_err(|e| {
        ErrorEnvelope::user_error(
            format!("write temp: {e}"),
            None,
            None,
            "could not write entity JSON",
        )
    })?;
    Ok(tmp.into_temp_path())
}

/**
 * Collects unique first-segment prefixes from discovered `.hbs` templates.
 */
pub fn list_type_prefixes(templates_root: &Path) -> Result<Vec<String>, ErrorEnvelope> {
    let entries = template_loader::discover_templates(templates_root, None)?;
    let mut prefixes: Vec<String> = entries
        .iter()
        .filter_map(|e| {
            e.rel_path
                .components()
                .next()
                .and_then(|c| c.as_os_str().to_str())
                .map(str::to_string)
        })
        .collect();
    prefixes.sort();
    prefixes.dedup();
    Ok(prefixes)
}

/**
 * Serializes a parsed template schema map for MCP JSON output.
 */
fn schema_to_json<T: serde::Serialize>(value: &T) -> Result<Value, ErrorEnvelope> {
    serde_json::to_value(value).map_err(|e| {
        ErrorEnvelope::user_error(
            format!("serialize schema: {e}"),
            None,
            None,
            "could not serialize template schema to JSON",
        )
    })
}

/**
 * Builds aggregated template description for agents.
 */
pub fn describe_templates(ctx: &ProjectContext) -> Result<Value, ErrorEnvelope> {
    let variables_schema = template_variables::load_schema(&ctx.templates_root)?;
    let field_types_schema = field_types::load_schema(&ctx.templates_root)?;
    let prefixes = list_type_prefixes(&ctx.templates_root)?;

    let runtime = RuntimeConfig::load(
        &project_setup_toml(&ctx.cwd),
        &project_setup_user_toml(&ctx.cwd),
    )?;

    Ok(json!({
        "templates_root": ctx.templates_root.display().to_string(),
        "type_prefixes": prefixes,
        "variables": schema_to_json(&variables_schema.0)?,
        "field_types": schema_to_json(&field_types_schema.types)?,
        "paths": {
            "lang": runtime.project.paths.lang,
            "aux": runtime.project.paths.aux,
        },
        "project": {
            "backend": runtime.project.project.backend.as_key(),
            "frontend": runtime.project.project.frontend.as_key(),
            "template": runtime
                .project
                .project
                .template
                .as_ref()
                .map(|t| format!("{t}")),
        },
    }))
}
