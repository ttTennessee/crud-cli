//! Discover `.crud/templates/` via `ignore::WalkBuilder` (D-G24, D-G31).

use globset::{Glob, GlobSetBuilder};
use ignore::WalkBuilder;
use std::path::{Path, PathBuf};

use super::error::ErrorEnvelope;
use super::i18n::{self, keys};
use super::template_variables::SCHEMA_FILE_NAME;
use super::type_map::TYPE_MAP_FILE_NAME;

/// One template file under `.crud/templates/`.
#[derive(Debug, Clone)]
pub struct TemplateEntry {
    /// Path relative to `.crud/templates/`.
    pub rel_path: PathBuf,
    /// Absolute path on disk.
    pub abs_path: PathBuf,
}

/**
 * Walks `templates_root` (either project-local `.crud/templates/` or a global
 * template under `~/.crud/templates/<name>/<version>/`) and returns the
 * rendered template entries.
 *
 * When `type_filter` is set, keeps templates whose `rel_path` matches a prefix.
 */
pub fn discover_templates(
    templates_root: &Path,
    type_filter: Option<&[String]>,
) -> Result<Vec<TemplateEntry>, ErrorEnvelope> {
    let root = templates_root.to_path_buf();
    if !root.is_dir() {
        return Err(no_templates_found(&root));
    }

    let mut entries = Vec::new();
    let walker = WalkBuilder::new(&root)
        .add_custom_ignore_filename(".crudignore")
        .build();

    for result in walker {
        let entry = result.map_err(|e| walk_error(&root, e.to_string()))?;
        let file_type = entry.file_type();
        if !file_type.is_some_and(|t| t.is_file()) {
            continue;
        }
        let abs = entry.path().to_path_buf();
        let rel = abs
            .strip_prefix(&root)
            .map_err(|_| walk_error(&root, "path outside template root".into()))?
            .to_path_buf();
        if rel
            .file_name()
            .is_some_and(|n| n == SCHEMA_FILE_NAME)
            && rel.parent().map(|p| p.as_os_str().is_empty()).unwrap_or(true)
        {
            continue;
        }
        if rel.file_name().is_some_and(|n| n == TYPE_MAP_FILE_NAME) {
            continue;
        }
        if rel.file_name().is_some_and(|n| n == "template.toml")
            && rel.parent().map(|p| p.as_os_str().is_empty()).unwrap_or(true)
        {
            continue;
        }
        entries.push(TemplateEntry {
            rel_path: rel,
            abs_path: abs,
        });
    }

    if let Some(prefixes) = type_filter {
        if !prefixes.is_empty() {
            entries = filter_by_type_prefixes(entries, prefixes)?;
            if entries.is_empty() {
                let available = scan_available_types(&root)?;
                let mut details = serde_json::Map::new();
                details.insert(
                    "requested".into(),
                    serde_json::Value::Array(
                        prefixes
                            .iter()
                            .map(|p| serde_json::Value::String(p.clone()))
                            .collect(),
                    ),
                );
                details.insert(
                    "available_types".into(),
                    serde_json::Value::Array(
                        available
                            .iter()
                            .map(|t| serde_json::Value::String(t.clone()))
                            .collect(),
                    ),
                );
                let hint = i18n::tf(
                    keys::ERROR_TEMPLATE_TYPE_NOT_FOUND,
                    &[("available", &available.join(", "))],
                );
                return Err(ErrorEnvelope::user_error_with_reason(
                    "template type not found",
                    "template_type_not_found",
                    details,
                    hint,
                ));
            }
        }
    }

    if entries.is_empty() {
        return Err(no_templates_found(&root));
    }

    entries.sort_by(|a, b| a.rel_path.cmp(&b.rel_path));
    Ok(entries)
}

fn normalize_rel_path(rel: &Path) -> String {
    rel.to_string_lossy().replace('\\', "/")
}

fn filter_by_type_prefixes(
    entries: Vec<TemplateEntry>,
    prefixes: &[String],
) -> Result<Vec<TemplateEntry>, ErrorEnvelope> {
    let mut builder = GlobSetBuilder::new();
    for prefix in prefixes {
        let p = prefix.trim().trim_matches('/');
        if p.is_empty() {
            continue;
        }
        builder.add(Glob::new(&format!("{p}/**")).map_err(glob_compile_err)?);
        builder.add(Glob::new(&format!("{p}*")).map_err(glob_compile_err)?);
    }
    let set = builder.build().map_err(glob_compile_err)?;

    Ok(entries
        .into_iter()
        .filter(|e| {
            let rel = normalize_rel_path(&e.rel_path);
            set.is_match(&rel)
        })
        .collect())
}

fn glob_compile_err(e: globset::Error) -> ErrorEnvelope {
    ErrorEnvelope::user_error(
        format!("invalid --type glob: {e}"),
        Some("type"),
        None,
        i18n::t(keys::ERROR_TEMPLATE_INVALID_TYPE_GLOB),
    )
}

/// Scans top-level and one nested directory under templates.
pub fn scan_available_types(root: &Path) -> Result<Vec<String>, ErrorEnvelope> {
    let mut types = Vec::new();
    let read = std::fs::read_dir(root).map_err(|e| walk_error(root, e.to_string()))?;
    for entry in read {
        let entry = entry.map_err(|e| walk_error(root, e.to_string()))?;
        let path = entry.path();
        if path.is_dir() {
            let name = entry.file_name().to_string_lossy().into_owned();
            types.push(name.clone());
            if let Ok(sub) = std::fs::read_dir(&path) {
                for sub_entry in sub {
                    let sub_entry = sub_entry.map_err(|e| walk_error(root, e.to_string()))?;
                    if sub_entry.path().is_dir() {
                        let sub_name = sub_entry.file_name().to_string_lossy().into_owned();
                        types.push(format!("{name}/{sub_name}"));
                    }
                }
            }
        }
    }
    types.sort();
    types.dedup();
    Ok(types)
}

fn no_templates_found(root: &Path) -> ErrorEnvelope {
    let mut details = serde_json::Map::new();
    details.insert(
        "path".into(),
        serde_json::Value::String(root.display().to_string()),
    );
    ErrorEnvelope::user_error_with_reason(
        "no templates in .crud/templates/",
        "no_templates_found",
        details,
        i18n::t(keys::ERROR_TEMPLATE_NO_TEMPLATES),
    )
}

fn walk_error(root: &Path, msg: String) -> ErrorEnvelope {
    let mut details = serde_json::Map::new();
    details.insert(
        "path".into(),
        serde_json::Value::String(root.display().to_string()),
    );
    details.insert("error".into(), serde_json::Value::String(msg));
    ErrorEnvelope::user_error_with_reason(
        "failed to walk templates directory",
        "template_walk_error",
        details,
        i18n::t(keys::ERROR_TEMPLATE_WALK_ERROR),
    )
}
