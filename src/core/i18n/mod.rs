//! Lightweight i18n: locale resolution + embedded TOML message catalogs.
//!
//! Design (route A+): zero external dependencies, full control over agent-mode
//! behavior, and a consistency test guarding both catalogs. Catalog files are
//! embedded via `include_str!` and parsed once into flat dotted-key maps.
//!
//! Locale precedence is resolved by the caller (see [`crate::cli`]); agent mode
//! always pins English so JSON output stays deterministic.

pub mod keys;

use std::collections::HashMap;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::OnceLock;

/// Supported UI languages.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lang {
    En,
    Zh,
}

impl Lang {
    /// Canonical short code persisted to config (`en` / `zh`).
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::En => "en",
            Self::Zh => "zh",
        }
    }

    /// Parses a user/config/env string into a [`Lang`]; `None` when unrecognized.
    #[must_use]
    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "en" | "en-us" | "en_us" | "english" => Some(Self::En),
            "zh" | "zh-cn" | "zh_cn" | "zh-hans" | "chinese" => Some(Self::Zh),
            _ => match s.trim() {
                "中文" | "简体中文" => Some(Self::Zh),
                _ => None,
            },
        }
    }
}

// 0 = En (default), 1 = Zh.
static CURRENT: AtomicU8 = AtomicU8::new(0);

/// Sets the active locale for subsequent lookups.
pub fn set(lang: Lang) {
    CURRENT.store(u8::from(lang == Lang::Zh), Ordering::Relaxed);
}

/// Returns the active locale (English by default).
#[must_use]
pub fn current() -> Lang {
    if CURRENT.load(Ordering::Relaxed) == 1 {
        Lang::Zh
    } else {
        Lang::En
    }
}

const EN_TOML: &str = include_str!("en.toml");
const ZH_TOML: &str = include_str!("zh.toml");

fn catalog(lang: Lang) -> &'static HashMap<String, String> {
    static EN: OnceLock<HashMap<String, String>> = OnceLock::new();
    static ZH: OnceLock<HashMap<String, String>> = OnceLock::new();
    match lang {
        Lang::En => EN.get_or_init(|| parse_catalog(EN_TOML)),
        Lang::Zh => ZH.get_or_init(|| parse_catalog(ZH_TOML)),
    }
}

fn parse_catalog(src: &str) -> HashMap<String, String> {
    // Catalogs are compiled in via include_str! and covered by
    // `locales_have_identical_key_sets`; a parse failure here means a
    // developer broke the bundled TOML, not anything a user can fix.
    #[allow(clippy::panic)]
    let value: toml::Value = src
        .parse()
        .unwrap_or_else(|e| panic!("i18n catalog is not valid TOML: {e}"));
    let mut out = HashMap::new();
    flatten("", &value, &mut out);
    out
}

fn flatten(prefix: &str, value: &toml::Value, out: &mut HashMap<String, String>) {
    match value {
        toml::Value::Table(table) => {
            for (k, v) in table {
                let key = if prefix.is_empty() {
                    k.clone()
                } else {
                    format!("{prefix}.{k}")
                };
                flatten(&key, v, out);
            }
        }
        toml::Value::String(s) => {
            out.insert(prefix.to_string(), s.clone());
        }
        // Non-string leaves are not valid messages; ignore.
        _ => {}
    }
}

/// Looks up `key` in the active locale, falling back to English, then to the
/// key itself (the last resort only triggers on a missing entry, which the
/// consistency test prevents from shipping).
#[must_use]
pub fn t(key: &str) -> &'static str {
    if let Some(s) = catalog(current()).get(key) {
        return s.as_str();
    }
    if let Some(s) = catalog(Lang::En).get(key) {
        return s.as_str();
    }
    Box::leak(key.to_string().into_boxed_str())
}

/// Like [`t`] but interpolates `{name}` placeholders from `args`.
#[must_use]
pub fn tf(key: &str, args: &[(&str, &str)]) -> String {
    let mut out = t(key).to_string();
    for (name, value) in args {
        out = out.replace(&format!("{{{name}}}"), value);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    #[test]
    fn locales_have_identical_key_sets() {
        let en = parse_catalog(EN_TOML);
        let zh = parse_catalog(ZH_TOML);
        let en_keys: BTreeSet<&String> = en.keys().collect();
        let zh_keys: BTreeSet<&String> = zh.keys().collect();
        assert_eq!(
            en_keys, zh_keys,
            "en.toml and zh.toml must define the same keys"
        );
    }

    #[test]
    fn every_referenced_key_has_entries() {
        let en = parse_catalog(EN_TOML);
        let zh = parse_catalog(ZH_TOML);
        for key in keys::ALL_KEYS {
            assert!(en.contains_key(*key), "en.toml missing key: {key}");
            assert!(zh.contains_key(*key), "zh.toml missing key: {key}");
        }
    }

    #[test]
    fn interpolation_replaces_named_placeholders() {
        set(Lang::En);
        let out = tf(keys::GEN_SUCCESS_WRITTEN, &[("count", "3")]);
        assert!(out.contains('3'));
        assert!(!out.contains("{count}"));
    }

    #[test]
    fn lang_parse_accepts_common_aliases() {
        assert_eq!(Lang::parse("EN"), Some(Lang::En));
        assert_eq!(Lang::parse("zh-CN"), Some(Lang::Zh));
        assert_eq!(Lang::parse("中文"), Some(Lang::Zh));
        assert_eq!(Lang::parse("fr"), None);
    }
}
