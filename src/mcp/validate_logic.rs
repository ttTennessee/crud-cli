//! Entity JSON validation without running the full generation pipeline.

use std::collections::BTreeMap;
use std::io::Write;
use std::path::Path;

use serde_json::{json, Value};
use tempfile::NamedTempFile;

use crate::core::config::RuntimeConfig;
use crate::core::error::ErrorEnvelope;
use crate::core::field_types;
use crate::core::gen_input::{load_gen_input_with_specs_from_json, GenCliOverrides};
use crate::core::paths::{project_setup_toml, project_setup_user_toml};
use crate::core::template_loader;
use crate::core::template_variables;

use super::context::ProjectContext;

/**
 * Validates `entity_json` against the active template schemas (no file writes).
 */
pub fn validate_entity_json(
    ctx: &ProjectContext,
    entity_json: &str,
    cli_vars: &BTreeMap<String, Value>,
) -> Result<(), ErrorEnvelope> {
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

    let mut loaded = load_gen_input_with_specs_from_json(
        &path,
        GenCliOverrides::default(),
    )?;

    let field_type_schema = field_types::load_schema(&ctx.templates_root)?;
    field_types::normalize_and_validate(&field_type_schema, &mut loaded.input.fields)?;
    if let Some(sub) = loaded.input.sub.as_mut() {
        field_types::normalize_and_validate(&field_type_schema, &mut sub.fields)?;
    }

    let schema = template_variables::load_schema(&ctx.templates_root)?;
    let _merged = template_variables::merge_values(&schema, cli_vars, &loaded.variables)?;

    Ok(())
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
