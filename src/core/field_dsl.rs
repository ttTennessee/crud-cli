//! `--fields` micro-DSL parser (D-G01, D-G07, D-G08).

use serde::{Deserialize, Serialize};
use std::collections::HashSet;

use super::error::ErrorEnvelope;

/// One field from the `--fields` DSL or JSON loader (Plan 02).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Field {
    pub name: String,
    #[serde(rename = "type")]
    pub ty: String,
    pub is_pk: bool,
    pub nullable: bool,
}

const RESERVED: &[&str] = &[
    "model",
    "table",
    "package",
    "package_path",
    "fields",
    "model_snake",
    "model_pascal",
    "model_camel",
    "model_kebab",
];

/**
 * Parses `src` into fields (`[*]name:Type[?]`, comma-separated).
 *
 * Fail-fast: returns the first validation error (D-G07).
 */
pub fn parse_fields(src: &str) -> Result<Vec<Field>, ErrorEnvelope> {
    let trimmed = src.trim();
    if trimmed.is_empty() {
        return Err(err_no_fields());
    }

    let mut seen = HashSet::new();
    let mut out = Vec::new();

    for raw_token in trimmed.split(',') {
        let token = raw_token.trim();
        if token.is_empty() {
            continue;
        }

        let (mut rest, is_pk) = if let Some(stripped) = token.strip_prefix('*') {
            (stripped, true)
        } else {
            (token, false)
        };

        let colon = rest.find(':').ok_or_else(|| {
            if rest.is_empty() {
                err_empty_name()
            } else {
                err_empty_type()
            }
        })?;

        let name_part = rest[..colon].trim();
        rest = rest[colon + 1..].trim();

        if rest.contains(':') {
            return Err(err_too_many_segments());
        }

        let (ty_part, nullable) = if let Some(stripped) = rest.strip_suffix('?') {
            (stripped.trim(), true)
        } else {
            (rest, false)
        };

        if name_part.is_empty() {
            return Err(err_empty_name());
        }
        if ty_part.is_empty() {
            return Err(err_empty_type());
        }

        if !valid_identifier(name_part) {
            return Err(err_invalid_identifier());
        }

        if RESERVED.contains(&name_part) {
            return Err(err_reserved_field_name(name_part));
        }

        if !seen.insert(name_part.to_string()) {
            return Err(err_duplicate_field());
        }

        out.push(Field {
            name: name_part.to_string(),
            ty: ty_part.to_string(),
            is_pk,
            nullable,
        });
    }

    if out.is_empty() {
        return Err(err_no_fields());
    }

    Ok(out)
}

fn valid_identifier(name: &str) -> bool {
    let mut chars = name.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !first.is_ascii_alphabetic() {
        return false;
    }
    chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

fn err_empty_type() -> ErrorEnvelope {
    ErrorEnvelope::user_error_with_reason(
        "field missing type",
        "empty_type",
        serde_json::Map::new(),
        "字段缺类型：写成 name:Type（如 email:String）",
    )
}

fn err_empty_name() -> ErrorEnvelope {
    ErrorEnvelope::user_error_with_reason(
        "field missing name",
        "empty_name",
        serde_json::Map::new(),
        "字段缺名称：写成 name:Type（如 email:String）",
    )
}

fn err_invalid_identifier() -> ErrorEnvelope {
    ErrorEnvelope::user_error_with_reason(
        "invalid field name",
        "invalid_identifier",
        serde_json::Map::new(),
        "字段名必须以字母开头、仅含字母数字和下划线（如 first_name）",
    )
}

fn err_too_many_segments() -> ErrorEnvelope {
    ErrorEnvelope::user_error_with_reason(
        "too many colons in field token",
        "too_many_segments",
        serde_json::Map::new(),
        "字段格式 name:Type，最多一个冒号（去掉额外的 :extra）",
    )
}

fn err_duplicate_field() -> ErrorEnvelope {
    ErrorEnvelope::user_error_with_reason(
        "duplicate field name",
        "duplicate_field",
        serde_json::Map::new(),
        "字段名重复：每个字段必须唯一",
    )
}

fn err_no_fields() -> ErrorEnvelope {
    ErrorEnvelope::user_error_with_reason(
        "no fields provided",
        "no_fields",
        serde_json::Map::new(),
        "--fields 不能为空，至少提供一个 name:Type",
    )
}

fn err_reserved_field_name(name: &str) -> ErrorEnvelope {
    let mut details = serde_json::Map::new();
    details.insert("name".into(), serde_json::Value::String(name.to_string()));
    ErrorEnvelope::user_error_with_reason(
        format!("reserved field name: {name}"),
        "reserved_field_name",
        details,
        "字段名与内置变量冲突：避开 model/table/package/package_path/fields/model_*",
    )
}
