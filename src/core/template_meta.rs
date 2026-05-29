//! YAML front-matter parsing for `.hbs` templates (D-G28 layer 1).

use gray_matter::{engine::YAML, Matter, Pod};

use super::config::OverwritePolicy;
use super::error::ErrorEnvelope;

/// Parsed front-matter keys (unknown keys ignored in Wave 2).
#[derive(Debug, Clone, Default)]
pub struct TemplateMeta {
    pub base_path: Option<String>,
    pub filename: Option<String>,
    pub overwrite: Option<OverwritePolicy>,
    /// Render only when this Handlebars condition is truthy (`generateWhen`).
    /// The value is the inside of an `{{#if ...}}`, e.g. `has_import` or
    /// `(eq mode "full")` — no surrounding `{{ }}`. Mutually exclusive with
    /// [`Self::skip_when`].
    pub generate_when: Option<String>,
    /// Skip rendering when this Handlebars condition is truthy (`skipWhen`).
    /// Inverse of [`Self::generate_when`]; the two cannot both be set.
    pub skip_when: Option<String>,
}

/**
 * Splits `---` YAML front-matter from template body (D-G29).
 *
 * Returns defaults and the full source when no opener is present.
 */
pub fn split_front_matter(src: &str) -> Result<(TemplateMeta, String), ErrorEnvelope> {
    if !src.starts_with("---") {
        return Ok((TemplateMeta::default(), src.to_string()));
    }

    let matter = Matter::<YAML>::new();
    let parsed = matter.parse(src);

    let data = match parsed.data {
        Some(d) if !d.is_empty() => d,
        _ => {
            if has_unclosed_front_matter(src) {
                return Err(ErrorEnvelope::template_error(
                    "malformed template front-matter YAML",
                ));
            }
            if has_closed_front_matter(src) {
                return Err(ErrorEnvelope::template_error(
                    "invalid YAML in template front-matter (quote values containing {{)",
                ));
            }
            return Ok((TemplateMeta::default(), parsed.content));
        }
    };

    let Pod::Hash(map) = &data else {
        return Err(ErrorEnvelope::template_error(
            "front-matter must be a YAML mapping",
        ));
    };

    let base_path = pod_string(map, &["basePath", "base_path"]);
    let filename = pod_string(map, &["filename"]);
    let overwrite = pod_string(map, &["overwrite"])
        .as_deref()
        .map(parse_overwrite_policy)
        .transpose()?;
    let generate_when = pod_string(map, &["generateWhen", "generate_when"]);
    let skip_when = pod_string(map, &["skipWhen", "skip_when"]);

    if generate_when.is_some() && skip_when.is_some() {
        return Err(ErrorEnvelope::template_error(
            "front-matter sets both generateWhen and skipWhen; use only one",
        ));
    }

    let meta = TemplateMeta {
        base_path,
        filename,
        overwrite,
        generate_when,
        skip_when,
    };

    Ok((meta, parsed.content))
}

fn pod_string(map: &std::collections::HashMap<String, Pod>, keys: &[&str]) -> Option<String> {
    for key in keys {
        if let Some(Pod::String(s)) = map.get(*key) {
            return Some(s.clone());
        }
    }
    None
}

fn has_unclosed_front_matter(src: &str) -> bool {
    let rest = src.trim_start().strip_prefix("---").unwrap_or(src);
    rest.contains('\n') && !rest.contains("\n---")
}

fn has_closed_front_matter(src: &str) -> bool {
    let rest = src.trim_start().strip_prefix("---").unwrap_or(src);
    rest.contains("\n---")
}

fn parse_overwrite_policy(raw: &str) -> Result<OverwritePolicy, ErrorEnvelope> {
    match raw.trim() {
        "never" => Ok(OverwritePolicy::Never),
        "force-only" => Ok(OverwritePolicy::ForceOnly),
        "always" => Ok(OverwritePolicy::Always),
        other => Err(ErrorEnvelope::template_error(format!(
            "unknown overwrite policy in front-matter: {other}"
        ))),
    }
}
