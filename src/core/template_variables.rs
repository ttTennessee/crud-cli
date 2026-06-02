//! `_variables.toml` schema — per-template-set variable declarations.
//!
//! Lives at `<templates_root>/_variables.toml` (project `.crud/templates/` or
//! `~/.crud/templates/<name>/<version>/` when a global bundle is pinned).
//! Declares the set of per-call
//! variables a template family expects (e.g. `has_import`, `btn_permission`).
//! Values come at gen time via `--var k=v` or JSON `variables`; this file is
//! the schema that validators and agents read.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::error::ErrorEnvelope;
use super::field_dsl::RESERVED_VARIABLE_NAMES;
use super::i18n::{self, keys};

/// File name of the schema (relative to `.crud/templates/`).
pub const SCHEMA_FILE_NAME: &str = "_variables.toml";

/// Declared type for a schema variable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum VarType {
    Bool,
    String,
    Number,
}

impl VarType {
    pub fn as_str(self) -> &'static str {
        match self {
            VarType::Bool => "bool",
            VarType::String => "string",
            VarType::Number => "number",
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct VariableDef {
    pub description: String,
    #[serde(rename = "type")]
    pub ty: VarType,
    #[serde(default)]
    pub required: bool,
    #[serde(default)]
    pub default: Option<toml::Value>,
}

/// Map of variable name → declaration. Iteration is deterministic via BTreeMap.
#[derive(Debug, Clone, Default)]
pub struct VariableSchema(pub BTreeMap<String, VariableDef>);

impl VariableSchema {
    pub fn names(&self) -> impl Iterator<Item = &str> {
        self.0.keys().map(String::as_str)
    }
}

/// Returns `_variables.toml` under a template bundle root.
pub fn schema_path(templates_root: &Path) -> PathBuf {
    templates_root.join(SCHEMA_FILE_NAME)
}

/// Loads `_variables.toml` from `templates_root` if present; absent file returns an empty schema.
pub fn load_schema(templates_root: &Path) -> Result<VariableSchema, ErrorEnvelope> {
    let path = schema_path(templates_root);
    if !path.exists() {
        return Ok(VariableSchema::default());
    }
    let raw = std::fs::read_to_string(&path).map_err(|e| {
        schema_error(format!("read {}: {e}", path.display()), "schema_read_error")
    })?;
    let parsed: BTreeMap<String, VariableDef> = toml::from_str(&raw)
        .map_err(|e| schema_error(format!("parse {}: {e}", path.display()), "schema_parse_error"))?;

    for (name, def) in &parsed {
        validate_name(name)?;
        if def.description.trim().is_empty() {
            return Err(schema_error(
                format!("variable {name}: description must not be empty"),
                "schema_missing_description",
            ));
        }
        if let Some(ref dv) = def.default {
            check_toml_matches_type(name, dv, def.ty)?;
        }
    }

    Ok(VariableSchema(parsed))
}

fn validate_name(name: &str) -> Result<(), ErrorEnvelope> {
    if name.trim().is_empty() {
        return Err(schema_error(
            "variable name must not be empty",
            "schema_empty_name",
        ));
    }
    if RESERVED_VARIABLE_NAMES.contains(&name) {
        return Err(schema_error(
            format!("variable {name} shadows a built-in"),
            "schema_reserved_name",
        ));
    }
    Ok(())
}

/// Resolves the final variable values for a gen run.
///
/// Priority: cli > json > schema.default. Missing required → error. Each
/// supplied value is type-checked against the schema. Undeclared CLI/JSON
/// keys are rejected with `unknown_variable`.
pub fn merge_values(
    schema: &VariableSchema,
    cli: &BTreeMap<String, Value>,
    json: &BTreeMap<String, Value>,
) -> Result<BTreeMap<String, Value>, ErrorEnvelope> {
    for key in cli.keys().chain(json.keys()) {
        if !schema.0.contains_key(key) {
            let mut details = serde_json::Map::new();
            details.insert("variable".into(), Value::String(key.clone()));
            return Err(ErrorEnvelope::user_error_with_reason(
                format!("undeclared variable: {key}"),
                "undeclared_variable",
                details,
                i18n::tf(
                    keys::ERROR_VARIABLE_UNDECLARED,
                    &[("key", key), ("schema_file", SCHEMA_FILE_NAME)],
                ),
            ));
        }
    }

    let mut out: BTreeMap<String, Value> = BTreeMap::new();
    for (name, def) in &schema.0 {
        let (value_opt, source) = if let Some(v) = cli.get(name) {
            (Some(v.clone()), "cli")
        } else if let Some(v) = json.get(name) {
            (Some(v.clone()), "json")
        } else if let Some(ref dv) = def.default {
            (Some(toml_to_json(dv)), "default")
        } else {
            (None, "missing")
        };

        match value_opt {
            Some(v) => {
                check_json_matches_type(name, &v, def.ty, source)?;
                out.insert(name.clone(), v);
            }
            None => {
                if def.required {
                    let mut details = serde_json::Map::new();
                    details.insert("variable".into(), Value::String(name.clone()));
                    return Err(ErrorEnvelope::user_error_with_reason(
                        format!("missing required variable: {name}"),
                        "missing_required_variable",
                        details,
                        i18n::tf(
                            keys::ERROR_VARIABLE_MISSING_REQUIRED,
                            &[("name", name.as_str())],
                        ),
                    ));
                }
                out.insert(name.clone(), Value::Null);
            }
        }
    }
    Ok(out)
}

fn toml_to_json(v: &toml::Value) -> Value {
    match v {
        toml::Value::String(s) => Value::String(s.clone()),
        toml::Value::Integer(i) => Value::Number((*i).into()),
        toml::Value::Float(f) => serde_json::Number::from_f64(*f)
            .map(Value::Number)
            .unwrap_or(Value::Null),
        toml::Value::Boolean(b) => Value::Bool(*b),
        toml::Value::Datetime(d) => Value::String(d.to_string()),
        toml::Value::Array(a) => Value::Array(a.iter().map(toml_to_json).collect()),
        toml::Value::Table(t) => {
            let mut m = serde_json::Map::new();
            for (k, val) in t {
                m.insert(k.clone(), toml_to_json(val));
            }
            Value::Object(m)
        }
    }
}

fn check_toml_matches_type(
    name: &str,
    v: &toml::Value,
    ty: VarType,
) -> Result<(), ErrorEnvelope> {
    let ok = matches!(
        (ty, v),
        (VarType::Bool, toml::Value::Boolean(_))
            | (VarType::String, toml::Value::String(_))
            | (VarType::Number, toml::Value::Integer(_))
            | (VarType::Number, toml::Value::Float(_))
    );
    if ok {
        Ok(())
    } else {
        Err(schema_error(
            format!(
                "variable {name}: default value type does not match declared type `{}`",
                ty.as_str()
            ),
            "schema_default_type_mismatch",
        ))
    }
}

fn check_json_matches_type(
    name: &str,
    v: &Value,
    ty: VarType,
    source: &str,
) -> Result<(), ErrorEnvelope> {
    let ok = matches!(
        (ty, v),
        (VarType::Bool, Value::Bool(_))
            | (VarType::String, Value::String(_))
            | (VarType::Number, Value::Number(_))
    );
    if ok {
        return Ok(());
    }
    let mut details = serde_json::Map::new();
    details.insert("variable".into(), Value::String(name.to_string()));
    details.insert("expected".into(), Value::String(ty.as_str().to_string()));
    details.insert("source".into(), Value::String(source.to_string()));
    Err(ErrorEnvelope::user_error_with_reason(
        format!("variable {name}: expected {} from {source}", ty.as_str()),
        "variable_type_mismatch",
        details,
        i18n::tf(
            keys::ERROR_VARIABLE_TYPE_MISMATCH,
            &[("expected", ty.as_str()), ("name", name)],
        ),
    ))
}

fn schema_error(msg: impl Into<String>, reason: &'static str) -> ErrorEnvelope {
    ErrorEnvelope::template_error_with_reason(
        msg,
        reason_details(reason),
        i18n::t(keys::ERROR_VARIABLE_SCHEMA_FIX),
    )
}

fn reason_details(reason: &'static str) -> serde_json::Map<String, Value> {
    let mut m = serde_json::Map::new();
    m.insert("reason".into(), Value::String(reason.to_string()));
    m
}

/// Parses a `--var key=value` argument; values are interpreted as JSON when
/// they parse as JSON literals (true/false/numbers/quoted strings), else as a
/// string.
pub fn parse_var_arg(raw: &str) -> Result<(String, Value), ErrorEnvelope> {
    let (k, v) = raw
        .split_once('=')
        .ok_or_else(|| bad_var_arg(raw, "expected key=value"))?;
    let key = k.trim().to_string();
    if key.is_empty() {
        return Err(bad_var_arg(raw, "empty key"));
    }
    let value = parse_var_value(v);
    Ok((key, value))
}

fn parse_var_value(raw: &str) -> Value {
    let trimmed = raw.trim();
    match trimmed {
        "true" => return Value::Bool(true),
        "false" => return Value::Bool(false),
        _ => {}
    }
    if let Ok(i) = trimmed.parse::<i64>() {
        return Value::Number(i.into());
    }
    if let Ok(f) = trimmed.parse::<f64>() {
        if let Some(n) = serde_json::Number::from_f64(f) {
            return Value::Number(n);
        }
    }
    Value::String(trimmed.to_string())
}

fn bad_var_arg(raw: &str, why: &'static str) -> ErrorEnvelope {
    let mut details = serde_json::Map::new();
    details.insert("input".into(), Value::String(raw.to_string()));
    details.insert("reason".into(), Value::String(why.to_string()));
    ErrorEnvelope::user_error_with_reason(
        format!("invalid --var: {raw}"),
        "invalid_var_arg",
        details,
        i18n::t(keys::ERROR_VARIABLE_INVALID_VAR_ARG),
    )
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]

    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn load_schema_reads_bundle_root_not_project_crud_templates() {
        let dir = TempDir::new().expect("tmp");
        let bundle = dir.path().join("mytmpl").join("1.0.0");
        fs::create_dir_all(&bundle).expect("mkdir");
        fs::write(
            bundle.join(SCHEMA_FILE_NAME),
            "[has_import]\ndescription = \"toggle\"\ntype = \"bool\"\ndefault = true\n",
        )
        .expect("write schema");
        let schema = load_schema(&bundle).expect("load");
        assert!(schema.0.contains_key("has_import"));
        assert!(
            !dir.path().join(".crud").join("templates").exists(),
            "schema must not require project-local path"
        );
    }
}
