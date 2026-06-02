//! `_field_types.toml` schema — per-template-set allowed field type declarations.
//!
//! Lives at `<templates_root>/_field_types.toml` (project `.crud/templates/` or
//! a pinned global bundle). Declares canonical DSL types agents may use in JSON
//! or `--fields`, with optional aliases for normalization before render.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use strsim::levenshtein;

use super::error::ErrorEnvelope;
use super::field_dsl::{Field, RESERVED_VARIABLE_NAMES};
use super::i18n::{self, keys};
use super::type_map;

/// File name of the field-type schema (relative to template bundle root).
pub const SCHEMA_FILE_NAME: &str = "_field_types.toml";

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FieldTypeDef {
    pub description: String,
    #[serde(default)]
    pub aliases: Vec<String>,
}

/// Map of canonical type name → declaration. Iteration is deterministic.
#[derive(Debug, Clone, Default)]
pub struct FieldTypeSchema {
    pub types: BTreeMap<String, FieldTypeDef>,
    /// Flattened alias → canonical lookup built at load time.
    alias_to_canonical: BTreeMap<String, String>,
}

impl FieldTypeSchema {
    pub fn is_empty(&self) -> bool {
        self.types.is_empty()
    }

    pub fn canonical_names(&self) -> impl Iterator<Item = &str> {
        self.types.keys().map(String::as_str)
    }
}

/// Returns `_field_types.toml` under a template bundle root.
pub fn schema_path(templates_root: &Path) -> PathBuf {
    templates_root.join(SCHEMA_FILE_NAME)
}

/// Loads `_field_types.toml` from `templates_root` if present; absent file returns empty schema.
pub fn load_schema(templates_root: &Path) -> Result<FieldTypeSchema, ErrorEnvelope> {
    let path = schema_path(templates_root);
    if !path.exists() {
        return Ok(FieldTypeSchema::default());
    }
    let raw = std::fs::read_to_string(&path).map_err(|e| {
        schema_error(format!("read {}: {e}", path.display()), "schema_read_error")
    })?;
    let parsed: BTreeMap<String, FieldTypeDef> = toml::from_str(&raw).map_err(|e| {
        schema_error(format!("parse {}: {e}", path.display()), "schema_parse_error")
    })?;

    let mut alias_to_canonical = BTreeMap::new();
    for (canonical, def) in &parsed {
        validate_type_name(canonical)?;
        if def.description.trim().is_empty() {
            return Err(schema_error(
                format!("field type {canonical}: description must not be empty"),
                "schema_missing_description",
            ));
        }
        for alias in &def.aliases {
            if alias.trim().is_empty() {
                return Err(schema_error(
                    format!("field type {canonical}: alias must not be empty"),
                    "schema_empty_alias",
                ));
            }
            if alias == canonical {
                return Err(schema_error(
                    format!("field type {canonical}: alias must differ from canonical name"),
                    "schema_alias_equals_canonical",
                ));
            }
            if parsed.contains_key(alias) {
                return Err(schema_error(
                    format!("field type {canonical}: alias `{alias}` collides with a canonical name"),
                    "schema_duplicate_alias",
                ));
            }
            if let Some(prev) = alias_to_canonical.insert(alias.clone(), canonical.clone()) {
                return Err(schema_error(
                    format!(
                        "alias `{alias}` is declared for both {prev} and {canonical}"
                    ),
                    "schema_duplicate_alias",
                ));
            }
        }
    }

    Ok(FieldTypeSchema {
        types: parsed,
        alias_to_canonical,
    })
}

fn validate_type_name(name: &str) -> Result<(), ErrorEnvelope> {
    if name.trim().is_empty() {
        return Err(schema_error(
            "field type name must not be empty",
            "schema_empty_name",
        ));
    }
    if RESERVED_VARIABLE_NAMES.contains(&name) {
        return Err(schema_error(
            format!("field type {name} shadows a built-in variable"),
            "schema_reserved_name",
        ));
    }
    Ok(())
}

/**
 * Validates each field `type` against the schema and normalizes aliases to canonical names.
 *
 * No-op when the schema is empty (file absent).
 */
pub fn normalize_and_validate(
    schema: &FieldTypeSchema,
    fields: &mut [Field],
) -> Result<(), ErrorEnvelope> {
    if schema.is_empty() {
        return Ok(());
    }

    let allowed: Vec<&str> = schema.canonical_names().collect();
    for field in fields.iter_mut() {
        let raw = field.ty.clone();
        let canonical = resolve_canonical(schema, &raw).ok_or_else(|| {
            unsupported_type_error(&field.name, &raw, &allowed)
        })?;
        field.ty = canonical;
    }
    Ok(())
}

fn resolve_canonical(schema: &FieldTypeSchema, ty: &str) -> Option<String> {
    if schema.types.contains_key(ty) {
        return Some(ty.to_string());
    }
    schema.alias_to_canonical.get(ty).cloned()
}

fn unsupported_type_error(field_name: &str, ty: &str, allowed: &[&str]) -> ErrorEnvelope {
    let mut details = serde_json::Map::new();
    details.insert("field".into(), Value::String(field_name.to_string()));
    details.insert("type".into(), Value::String(ty.to_string()));
    details.insert(
        "allowed".into(),
        Value::Array(
            allowed
                .iter()
                .map(|s| Value::String((*s).to_string()))
                .collect(),
        ),
    );
    details.insert(
        "schema_file".into(),
        Value::String(SCHEMA_FILE_NAME.to_string()),
    );

    let allowed_list = allowed.join(", ");
    let hint = match did_you_mean(ty, allowed) {
        Some(cand) => i18n::tf(
            keys::ERROR_FIELD_TYPE_UNSUPPORTED_DID_YOU_MEAN,
            &[("name", field_name), ("type", ty), ("candidate", &cand), ("allowed", &allowed_list)],
        ),
        None => i18n::tf(
            keys::ERROR_FIELD_TYPE_UNSUPPORTED,
            &[("name", field_name), ("type", ty), ("allowed", &allowed_list)],
        ),
    };

    ErrorEnvelope::user_error_with_reason(
        format!("unsupported field type: {ty} on field {field_name}"),
        "unsupported_field_type",
        details,
        hint,
    )
}

fn did_you_mean(got: &str, candidates: &[&str]) -> Option<String> {
    let mut best: Option<(String, usize)> = None;
    for &cand in candidates {
        let dist = levenshtein(got, cand);
        if dist <= 2 {
            match best {
                Some((_, d)) if dist >= d => {}
                _ => best = Some((cand.to_string(), dist)),
            }
        }
    }
    best.map(|(name, _)| name)
}

fn schema_error(msg: impl Into<String>, reason: &'static str) -> ErrorEnvelope {
    ErrorEnvelope::template_error_with_reason(
        msg,
        reason_details(reason),
        i18n::t(keys::ERROR_FIELD_TYPE_SCHEMA_FIX),
    )
}

fn reason_details(reason: &'static str) -> serde_json::Map<String, Value> {
    let mut m = serde_json::Map::new();
    m.insert("reason".into(), Value::String(reason.to_string()));
    m
}

/// Checks that each canonical type appears in at least one bundle `type_map.toml`.
///
/// Returns human-readable suggestions for validate issues; empty when schema is absent
/// or every type is covered by at least one bundle map.
pub fn type_map_coverage_issues(
    templates_root: &Path,
    schema: &FieldTypeSchema,
) -> Result<Vec<String>, ErrorEnvelope> {
    if schema.is_empty() {
        return Ok(Vec::new());
    }

    let bundle_maps = load_bundle_type_maps(templates_root)?;
    if bundle_maps.is_empty() {
        return Ok(Vec::new());
    }

    let mut issues = Vec::new();
    for canonical in schema.canonical_names() {
        let covered = bundle_maps
            .iter()
            .any(|(_bundle, map)| map.as_ref().is_some_and(|m| m.contains_key(canonical)));
        if !covered {
            let bundles: Vec<&str> = bundle_maps.iter().map(|(b, _)| b.as_str()).collect();
            issues.push(i18n::tf(
                keys::ERROR_FIELD_TYPE_UNMAPPED_IN_BUNDLES,
                &[
                    ("type", canonical),
                    ("bundles", &bundles.join(", ")),
                ],
            ));
        }
    }
    Ok(issues)
}

/// One bundle directory and its optional parsed `type_map.toml`.
type BundleTypeMap = (String, Option<BTreeMap<String, String>>);

fn load_bundle_type_maps(
    templates_root: &Path,
) -> Result<Vec<BundleTypeMap>, ErrorEnvelope> {
    let mut out = Vec::new();
    let Ok(entries) = std::fs::read_dir(templates_root) else {
        return Ok(out);
    };
    for entry in entries.flatten() {
        if !entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
            continue;
        }
        let Some(name) = entry.file_name().to_str().map(str::to_owned) else {
            continue;
        };
        if name.starts_with('_') || name.starts_with('.') {
            continue;
        }
        let map_path = entry.path().join(type_map::TYPE_MAP_FILE_NAME);
        if !map_path.is_file() {
            continue;
        }
        let map = type_map::load_for_bundle(templates_root, &name)?;
        out.push((name, map));
    }
    out.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(out)
}

/// Collects all canonical + alias strings declared in the schema (for tests).
#[cfg(test)]
pub fn all_declared_names(schema: &FieldTypeSchema) -> std::collections::BTreeSet<String> {
    use std::collections::BTreeSet;
    let mut set = BTreeSet::new();
    for (canonical, def) in &schema.types {
        set.insert(canonical.clone());
        for alias in &def.aliases {
            set.insert(alias.clone());
        }
    }
    set
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]

    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn load_schema_reads_types_and_aliases() {
        let dir = TempDir::new().expect("tmp");
        fs::write(
            dir.path().join(SCHEMA_FILE_NAME),
            "[Long]\ndescription = \"64-bit int\"\n\n[Integer]\ndescription = \"32-bit int\"\naliases = [\"int\"]\n",
        )
        .expect("write");
        let schema = load_schema(dir.path()).expect("load");
        assert!(schema.types.contains_key("Long"));
        assert_eq!(schema.alias_to_canonical.get("int").map(String::as_str), Some("Integer"));
    }

    #[test]
    fn normalize_maps_alias_to_canonical() {
        let dir = TempDir::new().expect("tmp");
        fs::write(
            dir.path().join(SCHEMA_FILE_NAME),
            "[Integer]\ndescription = \"32-bit int\"\naliases = [\"int\"]\n",
        )
        .expect("write");
        let schema = load_schema(dir.path()).expect("load");
        let mut fields = vec![Field {
            name: "age".into(),
            ty: "int".into(),
            is_pk: false,
            nullable: false,
        }];
        normalize_and_validate(&schema, &mut fields).expect("ok");
        assert_eq!(fields[0].ty, "Integer");
    }

    #[test]
    fn unknown_type_rejected_with_allowed_list() {
        let dir = TempDir::new().expect("tmp");
        fs::write(
            dir.path().join(SCHEMA_FILE_NAME),
            "[Long]\ndescription = \"pk\"\n",
        )
        .expect("write");
        let schema = load_schema(dir.path()).expect("load");
        let mut fields = vec![Field {
            name: "x".into(),
            ty: "String".into(),
            is_pk: false,
            nullable: false,
        }];
        let err = normalize_and_validate(&schema, &mut fields).expect_err("err");
        assert_eq!(
            err.details.get("reason").and_then(|v| v.as_str()),
            Some("unsupported_field_type")
        );
        assert!(err.details.get("allowed").is_some());
    }

    #[test]
    fn empty_schema_skips_validation() {
        let dir = TempDir::new().expect("tmp");
        let schema = load_schema(dir.path()).expect("load");
        assert!(schema.is_empty());
        let mut fields = vec![Field {
            name: "x".into(),
            ty: "Anything".into(),
            is_pk: false,
            nullable: false,
        }];
        normalize_and_validate(&schema, &mut fields).expect("ok");
        assert_eq!(fields[0].ty, "Anything");
    }
}
