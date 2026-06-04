//! Project cwd and template-root resolution for MCP handlers.

use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::core::config::RuntimeConfig;
use crate::core::error::ErrorEnvelope;
use crate::core::paths::{project_setup_toml, project_setup_user_toml};
use crate::core::template_loader;

/// Safety cap on parent-directory hops when searching for `.crud/setup.toml`.
const MAX_ASCENT_DEPTH: usize = 64;

/// How long to wait for the MCP client's `roots/list` response before falling back to cwd.
pub const ROOTS_LIST_TIMEOUT: Duration = Duration::from_secs(5);

/**
 * Loaded project runtime: merged setup + resolved template bundle root.
 */
#[derive(Clone)]
pub struct ProjectContext {
    pub cwd: PathBuf,
    pub templates_root: PathBuf,
}

/**
 * Walks upward from `start` looking for `.crud/setup.toml`.
 *
 * Stops at:
 * - the first directory that contains the file;
 * - the filesystem / volume root (`parent == current`);
 * - **home ceiling**: when `start` is under the user home directory, do not
 *   ascend above home (avoids scanning `/home` or `C:\Users` siblings);
 * - [`MAX_ASCENT_DEPTH`] hops.
 *
 * When `start` is not under home (e.g. `D:\work` on Windows while home is
 * `C:\Users\…`), only the volume root applies — home does not limit the search.
 */
#[must_use]
pub fn find_nearest_crud_root(start: &Path) -> Option<PathBuf> {
    let start = normalize_search_start(start);
    let home_ceiling = dirs::home_dir().filter(|home| path_is_within(&start, home));

    let mut current = start;
    for depth in 0..=MAX_ASCENT_DEPTH {
        if project_setup_toml(&current).is_file() {
            return Some(current);
        }

        let parent = current.parent()?;
        if parent == current {
            break;
        }
        if depth >= MAX_ASCENT_DEPTH {
            break;
        }
        if let Some(ref home) = home_ceiling {
            if !path_is_within(parent, home) {
                break;
            }
        }
        current = parent.to_path_buf();
    }
    None
}

/**
 * Converts a `file://` workspace root URI to a local path (MCP `roots/list`).
 */
#[must_use]
pub fn file_uri_to_path(uri: &str) -> PathBuf {
    let stripped = uri
        .strip_prefix("file://")
        .or_else(|| uri.strip_prefix("file:"))
        .unwrap_or(uri);
    let decoded = percent_decode_path(stripped);
    let path = PathBuf::from(decoded);
    if cfg!(windows) {
        let s = path.to_string_lossy();
        if s.starts_with('/') && s.len() >= 3 {
            let drive = &s[1..3];
            if drive.as_bytes()[1] == b':' {
                return PathBuf::from(format!("{}{}", drive, &s[3..]));
            }
        }
    }
    std::path::absolute(&path).unwrap_or(path)
}

fn percent_decode_path(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let bytes = input.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let (Some(hi), Some(lo)) = (hex_digit(bytes[i + 1]), hex_digit(bytes[i + 2])) {
                out.push(char::from(hi << 4 | lo));
                i += 3;
                continue;
            }
        }
        out.push(bytes[i] as char);
        i += 1;
    }
    out
}

fn hex_digit(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

/**
 * Resolves a project from `start` (or process cwd when `None`), walking up per
 * [`find_nearest_crud_root`], then loads setup + templates.
 */
pub fn load_project_context(start: Option<PathBuf>) -> Result<ProjectContext, ErrorEnvelope> {
    let searched_from =
        start.unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
    load_project_context_from_start(searched_from)
}

/**
 * Same as [`load_project_context`] but `start` is always explicit (tool override).
 */
pub fn load_project_context_from_start(
    searched_from: PathBuf,
) -> Result<ProjectContext, ErrorEnvelope> {
    let display_start = searched_from.display().to_string();
    let normalized = normalize_search_start(&searched_from);

    let project_root = find_nearest_crud_root(&normalized)
        .ok_or_else(|| mcp_project_not_found_error(&display_start, &normalized))?;

    let setup_path = project_setup_toml(&project_root);
    let runtime = RuntimeConfig::load(&setup_path, &project_setup_user_toml(&project_root))
        .map_err(|e| mcp_setup_load_error(&display_start, &project_root, &setup_path, e))?;

    let templates_root = template_loader::resolve_templates_root(&project_root, &runtime.project)
        .map_err(|e| mcp_templates_root_error(&project_root, e))?;

    Ok(ProjectContext {
        cwd: project_root,
        templates_root,
    })
}

fn normalize_search_start(path: &Path) -> PathBuf {
    std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

/// True when `path` is `prefix` or a descendant (platform-aware where possible).
fn path_is_within(path: &Path, prefix: &Path) -> bool {
    let path = normalize_search_start(path);
    let prefix = normalize_search_start(prefix);
    path.starts_with(&prefix)
}

fn mcp_project_not_found_error(searched_from: &str, normalized: &Path) -> ErrorEnvelope {
    let mut details = serde_json::Map::new();
    details.insert(
        "searched_from".into(),
        serde_json::Value::String(searched_from.to_string()),
    );
    details.insert(
        "normalized_start".into(),
        serde_json::Value::String(normalized.display().to_string()),
    );
    details.insert(
        "expected_marker".into(),
        serde_json::Value::String(".crud/setup.toml".into()),
    );

    let home_note = dirs::home_dir()
        .filter(|h| path_is_within(normalized, h))
        .map(|h| format!(" Search stopped at user home ({}).", h.display()))
        .unwrap_or_default();

    ErrorEnvelope::config_error_with_reason(
        format!("no crud-cli project found starting from {searched_from}"),
        "mcp_project_not_found",
        details,
        format!(
            "Working-directory detection issue: the MCP server could not find \
             `.crud/setup.toml` by walking up from the start path.{home_note} \
             Fix: (1) run `crud-cli setup --project` in your repo root; \
             (2) set MCP config `cwd` to that root, or add \
             `args: [\"mcp\", \"--path\", \"/absolute/path/to/project\"]` \
             (Cursor: use \"${{workspaceFolder}}\"); \
             (3) pass `project_root` to `crud_describe_templates` to probe another path."
        ),
    )
}

fn mcp_setup_load_error(
    searched_from: &str,
    project_root: &Path,
    setup_path: &Path,
    inner: ErrorEnvelope,
) -> ErrorEnvelope {
    let mut details = inner.details.clone();
    details.insert(
        "searched_from".into(),
        serde_json::Value::String(searched_from.to_string()),
    );
    details.insert(
        "project_root".into(),
        serde_json::Value::String(project_root.display().to_string()),
    );
    details.insert(
        "setup_path".into(),
        serde_json::Value::String(setup_path.display().to_string()),
    );

    ErrorEnvelope {
        kind: inner.kind,
        msg: format!(
            "found project at {} but failed to load {}: {}",
            project_root.display(),
            setup_path.display(),
            inner.msg
        ),
        exit_code: inner.exit_code,
        hint: format!(
            "A `.crud/setup.toml` exists at {} but could not be read or parsed. \
             Run `crud-cli setup --project --force` to regenerate, or fix the TOML. \
             Original hint: {}",
            project_root.display(),
            inner.hint
        ),
        details,
    }
}

fn mcp_templates_root_error(project_root: &Path, inner: ErrorEnvelope) -> ErrorEnvelope {
    let mut details = inner.details.clone();
    details.insert(
        "project_root".into(),
        serde_json::Value::String(project_root.display().to_string()),
    );

    ErrorEnvelope {
        kind: inner.kind,
        msg: format!(
            "project at {} loaded, but template bundle resolution failed: {}",
            project_root.display(),
            inner.msg
        ),
        exit_code: inner.exit_code,
        hint: format!(
            "Ensure `.crud/templates/` exists or run `crud-cli template use <name>`. {}",
            inner.hint
        ),
        details,
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn find_root_in_parent_directory() {
        let tmp = tempfile::tempdir().expect("tmpdir");
        let root = tmp.path().join("repo");
        let nested = root.join("packages").join("app");
        fs::create_dir_all(&nested).expect("mkdir");
        fs::create_dir_all(root.join(".crud")).expect("crud dir");
        fs::write(
            project_setup_toml(&root),
            "[project]\nbackend = \"none\"\nfrontend = \"none\"\n[paths]\nlang = \"src\"\naux = \"\"\n",
        )
        .expect("setup");

        let found = find_nearest_crud_root(&nested).expect("should find repo root");
        assert_eq!(found, normalize_search_start(&root));
    }

    #[test]
    fn find_root_does_not_escape_home_when_start_under_home() {
        let tmp = tempfile::tempdir().expect("tmpdir");
        let fake_home = tmp.path().join("home");
        let orphan = fake_home.join("orphan").join("deep");
        fs::create_dir_all(&orphan).expect("mkdir");
        // Project lives next to fake_home, not under orphan — simulates "wrong cwd under home"
        let project = tmp.path().join("real-project");
        fs::create_dir_all(project.join(".crud")).expect("crud");
        fs::write(
            project_setup_toml(&project),
            "[project]\nbackend = \"none\"\nfrontend = \"none\"\n",
        )
        .expect("setup");

        // Cannot see /tmp/real-project from orphan without leaving fake_home
        assert!(find_nearest_crud_root(&orphan).is_none());
    }

    #[test]
    fn file_uri_to_path_decodes_file_scheme() {
        let p = file_uri_to_path("file:///tmp/foo/bar");
        assert!(p.ends_with("tmp/foo/bar") || p.ends_with("tmp\\foo\\bar"));
    }
}
