//! Discover `.crud/templates/` via `ignore::WalkBuilder` (D-G24).

use ignore::WalkBuilder;
use std::path::{Path, PathBuf};

use super::error::ErrorEnvelope;

/// One template file under `.crud/templates/`.
#[derive(Debug, Clone)]
pub struct TemplateEntry {
    /// Path relative to `.crud/templates/`.
    pub rel_path: PathBuf,
    /// Absolute path on disk.
    pub abs_path: PathBuf,
}

/**
 * Walks `project_root/.crud/templates/` and returns template entries.
 *
 * `type_filter` is accepted for forward compatibility; Plan 02 applies filtering.
 */
pub fn discover_templates(
    project_root: &Path,
    _type_filter: Option<&[String]>,
) -> Result<Vec<TemplateEntry>, ErrorEnvelope> {
    // TODO(Plan-02): apply globset prefix filter from `type_filter`.
    let root = project_root.join(".crud/templates");
    if !root.is_dir() {
        return Err(no_templates_found(&root));
    }

  // Do not apply the repo's `.gitignore` to template discovery — templates may
  // intentionally live under paths ignored by the user's VCS.
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
        entries.push(TemplateEntry {
            rel_path: rel,
            abs_path: abs,
        });
    }

    if entries.is_empty() {
        return Err(no_templates_found(&root));
    }

    entries.sort_by(|a, b| a.rel_path.cmp(&b.rel_path));
    Ok(entries)
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
        "create .crud/templates/<name>.hbs or seed a template set",
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
        "check .crud/templates permissions and .crudignore syntax",
    )
}
