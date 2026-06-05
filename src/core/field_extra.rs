//! `_field_extra.toml` schema — per-template-set extra field attribute declarations.
//!
//! Lives at `<templates_root>/_field_extra.toml`. Declares which keys are valid
//! inside `fields[].extra` in entity JSON, along with their type, description,
//! and which field types require them. An absent file means no constraint on extra keys.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::error::ErrorEnvelope;
use super::gen_input::FieldSpec;
use super::i18n::{self, keys};

/// File name of the field-extra schema (relative to template bundle root).
pub const SCHEMA_FILE_NAME: &str = "_field_extra.toml";

/// Allowed value types for an extra key.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ExtraValueType {
    String,
    Number,
    Bool,
    Array,
    Object,
}

impl ExtraValueType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::String => "string",
            Self::Number => "number",
            Self::Bool => "bool",
            Self::Array => "array",
            Self::Object => "object",
        }
    }
}

/// Declaration of a single extra key.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FieldExtraDef {
    pub description: String,
    #[serde(rename = "type")]
    pub ty: ExtraValueType,
    /// Field types for which this extra key is required (empty = always optional).
    #[serde(default)]
    pub required_for: Vec<String>,
}

/// Map of extra key name → declaration. Iteration is deterministic via BTreeMap.
#[derive(Debug, Clone, Default)]
pub struct FieldExtraSchema(pub BTreeMap<String, FieldExtraDef>);

impl FieldExtraSchema {
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

/// Returns `_field_extra.toml` path under a template bundle root.
pub fn schema_path(templates_root: &Path) -> PathBuf {
    templates_root.join(SCHEMA_FILE_NAME)
}

/// Loads `_field_extra.toml` from `templates_root` if present; absent file returns empty schema.
pub fn load_schema(templates_root: &Path) -> Result<FieldExtraSchema, ErrorEnvelope> {
    let path = schema_path(templates_root);
    if !path.exists() {
        return Ok(FieldExtraSchema::default());
    }
    let raw = std::fs::read_to_string(&path)
        .map_err(|e| schema_error(format!("read {}: {e}", path.display()), "schema_read_error"))?;
    let parsed: BTreeMap<String, FieldExtraDef> = toml::from_str(&raw).map_err(|e| {
        schema_error(
            format!("parse {}: {e}", path.display()),
            "schema_parse_error",
        )
    })?;

    for (key, def) in &parsed {
        if key.trim().is_empty() {
            return Err(schema_error(
                "extra key name must not be empty",
                "schema_empty_key",
            ));
        }
        if def.description.trim().is_empty() {
            return Err(schema_error(
                format!("extra key `{key}`: description must not be empty"),
                "schema_missing_description",
            ));
        }
        for ty_name in &def.required_for {
            if ty_name.trim().is_empty() {
                return Err(schema_error(
                    format!("extra key `{key}`: required_for entry must not be empty"),
                    "schema_empty_required_for",
                ));
            }
        }
    }

    Ok(FieldExtraSchema(parsed))
}

/// Checks `FieldSpec.extra` keys against the schema.
///
/// Returns a list of human-readable problem descriptions; empty means no issues.
/// When the schema is empty (file absent) the check is skipped entirely.
pub fn validate_extra_keys(schema: &FieldExtraSchema, specs: &[FieldSpec]) -> Vec<String> {
    if schema.is_empty() {
        return Vec::new();
    }

    let mut problems = Vec::new();
    for spec in specs {
        // Unknown keys
        for key in spec.extra.keys() {
            if !schema.0.contains_key(key.as_str()) {
                problems.push(i18n::tf(
                    keys::ERROR_FIELD_EXTRA_UNKNOWN_KEY,
                    &[("field", spec.name.as_str()), ("key", key.as_str())],
                ));
            }
        }

        // required_for keys that are missing
        for (key, def) in &schema.0 {
            if def.required_for.contains(&spec.ty)
                && !spec.extra.contains_key(key.as_str())
            {
                problems.push(i18n::tf(
                    keys::ERROR_FIELD_EXTRA_MISSING_REQUIRED,
                    &[
                        ("field", spec.name.as_str()),
                        ("key", key.as_str()),
                        ("field_type", spec.ty.as_str()),
                    ],
                ));
            }
        }
    }
    problems
}

fn schema_error(msg: impl Into<String>, reason: &'static str) -> ErrorEnvelope {
    let mut details = serde_json::Map::new();
    details.insert("reason".into(), Value::String(reason.to_string()));
    ErrorEnvelope::template_error_with_reason(
        msg,
        details,
        i18n::t(keys::ERROR_FIELD_EXTRA_SCHEMA_FIX),
    )
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]

    use super::*;
    use serde_json::Map;
    use std::fs;
    use tempfile::TempDir;

    fn make_spec(name: &str, ty: &str, extra: &[(&str, Value)]) -> FieldSpec {
        let mut m = Map::new();
        for (k, v) in extra {
            m.insert((*k).to_string(), v.clone());
        }
        FieldSpec {
            name: name.to_string(),
            ty: ty.to_string(),
            is_pk: false,
            nullable: false,
            required: false,
            length: None,
            unique: false,
            default: None,
            comment: String::new(),
            extra: m,
        }
    }

    #[test]
    fn no_schema_skips_all_checks() {
        let dir = TempDir::new().expect("tmp");
        let schema = load_schema(dir.path()).expect("load");
        assert!(schema.is_empty());
        let spec = make_spec("foo", "String", &[("anything", Value::Bool(true))]);
        assert!(validate_extra_keys(&schema, &[spec]).is_empty());
    }

    #[test]
    fn known_key_passes() {
        let dir = TempDir::new().expect("tmp");
        fs::write(
            dir.path().join(SCHEMA_FILE_NAME),
            "[options]\ndescription = \"enum options\"\ntype = \"array\"\n",
        )
        .expect("write");
        let schema = load_schema(dir.path()).expect("load");
        let spec = make_spec("status", "enum", &[("options", Value::Array(vec![]))]);
        assert!(validate_extra_keys(&schema, &[spec]).is_empty());
    }

    #[test]
    fn unknown_key_reported() {
        let dir = TempDir::new().expect("tmp");
        fs::write(
            dir.path().join(SCHEMA_FILE_NAME),
            "[options]\ndescription = \"enum options\"\ntype = \"array\"\n",
        )
        .expect("write");
        let schema = load_schema(dir.path()).expect("load");
        let spec = make_spec("status", "enum", &[("unknown_key", Value::Bool(true))]);
        let problems = validate_extra_keys(&schema, &[spec]);
        assert_eq!(problems.len(), 1);
        assert!(problems[0].contains("unknown_key"));
    }

    #[test]
    fn required_for_missing_reported() {
        let dir = TempDir::new().expect("tmp");
        fs::write(
            dir.path().join(SCHEMA_FILE_NAME),
            "[options]\ndescription = \"enum options\"\ntype = \"array\"\nrequired_for = [\"enum\"]\n",
        )
        .expect("write");
        let schema = load_schema(dir.path()).expect("load");
        let spec = make_spec("status", "enum", &[]);
        let problems = validate_extra_keys(&schema, &[spec]);
        assert_eq!(problems.len(), 1);
        assert!(problems[0].contains("options"));
    }

    #[test]
    fn required_for_present_passes() {
        let dir = TempDir::new().expect("tmp");
        fs::write(
            dir.path().join(SCHEMA_FILE_NAME),
            "[options]\ndescription = \"enum options\"\ntype = \"array\"\nrequired_for = [\"enum\"]\n",
        )
        .expect("write");
        let schema = load_schema(dir.path()).expect("load");
        let spec = make_spec("status", "enum", &[("options", Value::Array(vec![]))]);
        assert!(validate_extra_keys(&schema, &[spec]).is_empty());
    }

    #[test]
    fn required_for_other_type_not_enforced() {
        let dir = TempDir::new().expect("tmp");
        fs::write(
            dir.path().join(SCHEMA_FILE_NAME),
            "[options]\ndescription = \"enum options\"\ntype = \"array\"\nrequired_for = [\"enum\"]\n",
        )
        .expect("write");
        let schema = load_schema(dir.path()).expect("load");
        // type is String, not enum — no requirement
        let spec = make_spec("name", "String", &[]);
        assert!(validate_extra_keys(&schema, &[spec]).is_empty());
    }

    #[test]
    fn empty_description_rejected() {
        let dir = TempDir::new().expect("tmp");
        fs::write(
            dir.path().join(SCHEMA_FILE_NAME),
            "[options]\ndescription = \"\"\ntype = \"array\"\n",
        )
        .expect("write");
        let err = load_schema(dir.path()).expect_err("should fail");
        assert!(err.msg.contains("description"));
    }
}
