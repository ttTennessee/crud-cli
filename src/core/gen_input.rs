//! In-memory generation input (DSL and JSON `--file` loader).

use std::io::ErrorKind;
use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use strsim::levenshtein;

use super::error::ErrorEnvelope;
use super::field_dsl::Field;

/// Entity input consumed by `build_context` and `gen_pipeline` (D-G13).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GenInput {
    pub name: String,
    pub table: String,
    pub package: String,
    pub fields: Vec<Field>,
}

/// JSON field shape (`--file`); converted to [`Field`] for [`GenInput`] (D-G15).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FieldSpec {
    pub name: String,
    #[serde(rename = "type")]
    pub ty: String,
    #[serde(default)]
    pub is_pk: bool,
    #[serde(default)]
    pub nullable: bool,
    #[serde(default)]
    pub length: Option<u32>,
    #[serde(default)]
    pub unique: bool,
    #[serde(default)]
    pub default: Option<Value>,
    #[serde(default)]
    pub comment: String,
    #[serde(default)]
    pub extra: Map<String, Value>,
}

/// On-disk JSON entity file (not the in-memory [`GenInput`]).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct JsonInputFile {
    pub name: String,
    pub table: String,
    pub package: String,
    pub fields: Vec<FieldSpec>,
}

/// CLI overrides for JSON input (D-G11); CLI wins over file values.
#[derive(Debug, Clone, Default)]
pub struct GenCliOverrides {
    pub name: Option<String>,
    pub package: Option<String>,
    pub table: Option<String>,
}

/// Loaded JSON bundle: canonical [`GenInput`] plus specs for rich context (Plan 02).
#[derive(Debug, Clone)]
pub struct JsonLoadResult {
    pub input: GenInput,
    pub field_specs: Vec<FieldSpec>,
}

const FIELD_SPEC_KEYS: &[&str] = &[
    "name", "type", "is_pk", "nullable", "length", "unique", "default", "comment", "extra",
];

/**
 * Loads `user.json`, applies CLI overrides, returns [`GenInput`] (D-G02).
 */
pub fn load_gen_input_from_json(
    path: &Path,
    overrides: GenCliOverrides,
) -> Result<GenInput, ErrorEnvelope> {
    Ok(load_gen_input_with_specs_from_json(path, overrides)?.input)
}

/// Same as [`load_gen_input_from_json`] but retains [`FieldSpec`] for context `extra` merge.
pub fn load_gen_input_with_specs_from_json(
    path: &Path,
    overrides: GenCliOverrides,
) -> Result<JsonLoadResult, ErrorEnvelope> {
    let raw = std::fs::read_to_string(path).map_err(|e| {
        if e.kind() == ErrorKind::NotFound {
            let mut details = serde_json::Map::new();
            details.insert("path".into(), Value::String(path.display().to_string()));
            return ErrorEnvelope::user_error_with_reason(
                format!("file not found: {}", path.display()),
                "file_not_found",
                details,
                "create the JSON file or pass --fields instead",
            );
        }
        json_error("invalid_json", "", &e.to_string(), "check JSON syntax")
    })?;

    let de = &mut serde_json::Deserializer::from_str(&raw);
    let parsed: JsonInputFile =
        serde_path_to_error::deserialize(de).map_err(map_json_deser_error)?;

    let name = resolve_scalar(
        overrides.name,
        parsed.name,
        "name",
        "provide --name or set \"name\" in JSON",
    )?;
    let package = resolve_scalar(
        overrides.package,
        parsed.package,
        "package",
        "provide --package or set \"package\" in JSON",
    )?;
    let table = resolve_scalar(
        overrides.table,
        parsed.table,
        "table",
        "provide --table or set \"table\" in JSON",
    )?;

    let fields: Vec<Field> = parsed
        .fields
        .iter()
        .map(field_spec_to_field)
        .collect();

    if fields.is_empty() {
        return Err(ErrorEnvelope::user_error_with_reason(
            "no fields in JSON",
            "no_fields",
            serde_json::Map::new(),
            "add at least one field object to \"fields\"",
        ));
    }

    Ok(JsonLoadResult {
        input: GenInput {
            name,
            table,
            package,
            fields,
        },
        field_specs: parsed.fields,
    })
}

fn resolve_scalar(
    cli: Option<String>,
    from_json: String,
    field: &'static str,
    hint: &'static str,
) -> Result<String, ErrorEnvelope> {
    let value = cli.unwrap_or(from_json);
    if value.trim().is_empty() {
        let mut details = serde_json::Map::new();
        details.insert("field".into(), Value::String(field.to_string()));
        let reason = match field {
            "name" => "missing_name",
            "package" => "missing_package",
            "table" => "missing_table",
            _ => "missing_field",
        };
        return Err(ErrorEnvelope::user_error_with_reason(
            format!("missing {field}"),
            reason,
            details,
            hint,
        ));
    }
    Ok(value)
}

fn field_spec_to_field(spec: &FieldSpec) -> Field {
    Field {
        name: spec.name.clone(),
        ty: spec.ty.clone(),
        is_pk: spec.is_pk,
        nullable: spec.nullable,
    }
}

fn map_json_deser_error(err: serde_path_to_error::Error<serde_json::Error>) -> ErrorEnvelope {
    let path = err.path().to_string();
    let inner = err.into_inner();
    let msg = inner.to_string();

    if inner.is_syntax() || inner.is_eof() {
        return json_error("invalid_json", &path, &msg, "check JSON syntax");
    }

    if msg.contains("unknown field") {
        let got = extract_unknown_field_token(&msg).unwrap_or_default();
        let mut details = serde_json::Map::new();
        if !path.is_empty() {
            details.insert("path".into(), Value::String(path.clone()));
        }
        details.insert("got".into(), Value::String(got.clone()));
        let expected: Vec<Value> = FIELD_SPEC_KEYS
            .iter()
            .map(|k| Value::String((*k).to_string()))
            .collect();
        details.insert("expected".into(), Value::Array(expected));
        let hint = did_you_mean_hint(&got, FIELD_SPEC_KEYS);
        return ErrorEnvelope::user_error_with_reason(
            format!("unknown field `{got}`"),
            "unknown_field",
            details,
            hint,
        );
    }

    if msg.contains("missing field") {
        let field = extract_quoted_token(&msg).unwrap_or_else(|| "field".to_string());
        let mut details = serde_json::Map::new();
        if !path.is_empty() {
            details.insert("path".into(), Value::String(path));
        }
        details.insert("field".into(), Value::String(field));
        return ErrorEnvelope::user_error_with_reason(
            msg,
            "missing_field",
            details,
            "add the missing property to your JSON",
        );
    }

    if msg.contains("invalid type") || msg.contains("expected") {
        let mut details = serde_json::Map::new();
        if !path.is_empty() {
            details.insert("path".into(), Value::String(path));
        }
        if let Some(got) = extract_got_type(&msg) {
            details.insert("got".into(), Value::String(got));
        }
        details.insert("expected".into(), Value::String("see serde message".into()));
        return ErrorEnvelope::user_error_with_reason(
            msg,
            "type_mismatch",
            details,
            "fix the field type in JSON",
        );
    }

    json_error("invalid_value", &path, &msg, "fix the value at the reported path")
}

fn json_error(reason: &'static str, path: &str, msg: &str, hint: &str) -> ErrorEnvelope {
    let mut details = serde_json::Map::new();
    if !path.is_empty() {
        details.insert("path".into(), Value::String(path.to_string()));
    }
    ErrorEnvelope::user_error_with_reason(msg, reason, details, hint)
}

fn did_you_mean_hint(got: &str, candidates: &[&str]) -> String {
    let mut best: Option<(&str, usize)> = None;
    for &cand in candidates {
        let dist = levenshtein(got, cand);
        if dist <= 2 {
            match best {
                Some((_, d)) if dist >= d => {}
                _ => best = Some((cand, dist)),
            }
        }
    }
    match best {
        Some((cand, _)) => format!("did you mean '{cand}'?"),
        None => "check field names against the JSON schema".into(),
    }
}

fn extract_unknown_field_token(msg: &str) -> Option<String> {
    extract_quoted_token(msg)
}

fn extract_quoted_token(msg: &str) -> Option<String> {
    let start = msg.find('`')? + 1;
    let rest = &msg[start..];
    let end = rest.find('`')?;
    Some(rest[..end].to_string())
}

fn extract_got_type(msg: &str) -> Option<String> {
    if msg.contains("string") {
        return Some("string".into());
    }
    if msg.contains("boolean") {
        return Some("bool".into());
    }
    None
}
