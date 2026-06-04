//! Per-template `type_map.toml` loader and global fallback policy.
//!
//! Each top-level bundle under `.crud/templates/<bundle>/` may carry a
//! `type_map.toml` declaring how neutral DSL types render in that stack.
//! Missing maps + missing keys both fall through to the global policy
//! configured in `setup.toml::[type_map].fallback`.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;

use super::error::ErrorEnvelope;
use super::i18n::{self, keys};

/// Sentinel filename for per-bundle map; filtered from template walk.
pub const TYPE_MAP_FILE_NAME: &str = "type_map.toml";

/// Global fallback policy when a type lookup misses.
#[derive(Debug, Clone, PartialEq, Eq)]
#[derive(Default)]
pub enum Fallback {
    /// Output the source type string unchanged (default; backward compatible).
    #[default]
    Passthrough,
    /// Treat unknown type as a user error and abort render.
    Error,
    /// Replace unknown type with a fixed literal (e.g., `"any"`).
    Literal(String),
}


impl Serialize for Fallback {
    fn serialize<S: serde::Serializer>(&self, ser: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::Passthrough => ser.serialize_str("passthrough"),
            Self::Error => ser.serialize_str("error"),
            Self::Literal(s) => ser.serialize_str(s),
        }
    }
}

impl<'de> Deserialize<'de> for Fallback {
    fn deserialize<D: serde::Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
        let s = String::deserialize(de)?;
        Ok(match s.as_str() {
            "passthrough" => Self::Passthrough,
            "error" => Self::Error,
            _ => Self::Literal(s),
        })
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct TypeMapFile {
    #[serde(default)]
    map: BTreeMap<String, String>,
}

/// Reads `<templates_root>/<bundle>/type_map.toml` if present.
///
/// Returns `Ok(None)` when the file does not exist (a deliberate absence —
/// behave as an empty map; every lookup falls through to the global policy).
pub fn load_for_bundle(
    templates_root: &Path,
    bundle: &str,
) -> Result<Option<BTreeMap<String, String>>, ErrorEnvelope> {
    let path = templates_root.join(bundle).join(TYPE_MAP_FILE_NAME);
    if !path.is_file() {
        return Ok(None);
    }
    let raw = std::fs::read_to_string(&path).map_err(|e| {
        ErrorEnvelope::user_error_with_reason(
            format!("read {}: {e}", path.display()),
            "type_map_read_failed",
            details_for(&path, bundle),
            i18n::t(keys::ERROR_TYPE_MAP_READ_FAILED),
        )
    })?;
    let parsed: TypeMapFile = toml::from_str(&raw).map_err(|e| {
        ErrorEnvelope::user_error_with_reason(
            format!("parse {}: {e}", path.display()),
            "type_map_parse_failed",
            details_for(&path, bundle),
            i18n::t(keys::ERROR_TYPE_MAP_PARSE_FAILED),
        )
    })?;
    Ok(Some(parsed.map))
}

/// Applies the lookup: explicit map hit → mapped; otherwise → fallback.
///
/// Returns `Err` only when fallback is `Error` and the type is unmapped.
pub fn resolve(
    bundle: Option<&str>,
    map: Option<&BTreeMap<String, String>>,
    ty: &str,
    fallback: &Fallback,
) -> Result<String, ErrorEnvelope> {
    if let Some(m) = map {
        if let Some(v) = m.get(ty) {
            return Ok(v.clone());
        }
    }
    match fallback {
        Fallback::Passthrough => Ok(ty.to_string()),
        Fallback::Literal(s) => Ok(s.clone()),
        Fallback::Error => Err(unmapped_type_error(bundle, ty)),
    }
}

fn unmapped_type_error(bundle: Option<&str>, ty: &str) -> ErrorEnvelope {
    let mut details = serde_json::Map::new();
    details.insert("type".into(), serde_json::Value::String(ty.to_string()));
    if let Some(b) = bundle {
        details.insert("bundle".into(), serde_json::Value::String(b.to_string()));
    }
    let hint = match bundle {
        Some(b) => i18n::tf(
            keys::ERROR_TYPE_MAP_UNMAPPED_BUNDLE,
            &[("bundle", b), ("type", ty)],
        ),
        None => i18n::tf(keys::ERROR_TYPE_MAP_UNMAPPED_GLOBAL, &[("type", ty)]),
    };
    ErrorEnvelope::user_error_with_reason(
        format!("unmapped type: {ty}"),
        "type_map_unmapped",
        details,
        hint,
    )
}

fn details_for(path: &Path, bundle: &str) -> serde_json::Map<String, serde_json::Value> {
    let mut d = serde_json::Map::new();
    d.insert("path".into(), serde_json::Value::String(path.display().to_string()));
    d.insert("bundle".into(), serde_json::Value::String(bundle.to_string()));
    d
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]
    use super::*;
    use tempfile::TempDir;

    fn write(path: &Path, body: &str) {
        std::fs::create_dir_all(path.parent().expect("parent")).expect("mkdir");
        std::fs::write(path, body).expect("write");
    }

    #[test]
    fn fallback_round_trips() {
        #[derive(Deserialize)]
        struct W { fallback: Fallback }
        let w: W = toml::from_str(r#"fallback = "passthrough""#).expect("p");
        assert_eq!(w.fallback, Fallback::Passthrough);
        let w: W = toml::from_str(r#"fallback = "error""#).expect("p");
        assert_eq!(w.fallback, Fallback::Error);
        let w: W = toml::from_str(r#"fallback = "any""#).expect("p");
        assert_eq!(w.fallback, Fallback::Literal("any".into()));
    }

    #[test]
    fn load_missing_returns_none() {
        let dir = TempDir::new().expect("tmp");
        assert!(load_for_bundle(dir.path(), "java").expect("ok").is_none());
    }

    #[test]
    fn load_parses_map() {
        let dir = TempDir::new().expect("tmp");
        let p = dir.path().join("java").join(TYPE_MAP_FILE_NAME);
        write(&p, "[map]\nint = \"Integer\"\nstring = \"String\"\n");
        let m = load_for_bundle(dir.path(), "java")
            .expect("ok")
            .expect("some");
        assert_eq!(m["int"], "Integer");
        assert_eq!(m["string"], "String");
    }

    #[test]
    fn resolve_hits_map() {
        let mut m = BTreeMap::new();
        m.insert("int".into(), "Integer".into());
        let out = resolve(Some("java"), Some(&m), "int", &Fallback::Error).expect("ok");
        assert_eq!(out, "Integer");
    }

    #[test]
    fn resolve_passthrough_on_miss() {
        let out = resolve(None, None, "Custom", &Fallback::Passthrough).expect("ok");
        assert_eq!(out, "Custom");
    }

    #[test]
    fn resolve_literal_on_miss() {
        let out = resolve(None, None, "Custom", &Fallback::Literal("any".into())).expect("ok");
        assert_eq!(out, "any");
    }

    #[test]
    fn resolve_error_on_miss() {
        let err = resolve(Some("ts"), None, "Custom", &Fallback::Error).expect_err("err");
        let msg = format!("{err:?}");
        assert!(msg.contains("Custom"));
    }
}
