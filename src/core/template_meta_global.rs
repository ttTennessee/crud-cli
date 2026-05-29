//! Global template registry: discovers templates installed at
//! `~/.crud/templates/<name>/<version>/` and parses their `template.toml`
//! manifest.
//!
//! A "template" here is the user-installed bundle that `crud-cli setup` and
//! `crud-cli template use` reference by name. Distinct from
//! [`template_meta`](super::template_meta) which parses front-matter inside
//! individual `.hbs` files.

use std::cmp::Ordering;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::config::{Backend, Frontend};
use super::error::{ErrorEnvelope, Kind};

/// File name of the per-template manifest.
pub const MANIFEST_FILENAME: &str = "template.toml";

/// Parsed `template.toml` payload.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TemplateManifest {
    /// Optional display name; defaults to the directory name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Optional version; defaults to the version directory name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    /// Backend language declared by the template.
    pub backend: Backend,
    /// Frontend language declared by the template.
    pub frontend: Frontend,
    /// Optional one-line description shown in `template list`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// A successfully-loaded installed template.
#[derive(Debug, Clone)]
pub struct InstalledTemplate {
    /// Resolved name (manifest > directory name).
    pub name: String,
    /// Resolved version (manifest > version directory name).
    pub version: String,
    /// Absolute path to the `<version>/` directory.
    pub path: PathBuf,
    /// Parsed manifest.
    pub manifest: TemplateManifest,
}

/// Returns `~/.crud/templates` for the current user. `None` when the home
/// directory cannot be resolved.
#[must_use]
pub fn global_templates_root() -> Option<PathBuf> {
    dirs::home_dir().map(|h| h.join(".crud").join("templates"))
}

/// Enumerates every installed `<name>/<version>/` directory under
/// `global_templates_root()` that has a parseable `template.toml`.
///
/// Versions missing a manifest are skipped with a `tracing::warn!`. Returns an
/// empty vec when the root is absent.
#[must_use]
pub fn list_installed_templates() -> Vec<InstalledTemplate> {
    let Some(root) = global_templates_root() else {
        return Vec::new();
    };
    list_installed_in(&root)
}

/// Same as [`list_installed_templates`] but rooted at an explicit directory —
/// used by tests that redirect `~/.crud` to a tempdir.
#[must_use]
pub fn list_installed_in(root: &Path) -> Vec<InstalledTemplate> {
    let mut out = Vec::new();
    let Ok(entries) = std::fs::read_dir(root) else {
        return out;
    };
    for name_entry in entries.flatten() {
        if !name_entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
            continue;
        }
        let name_dir = name_entry.path();
        let Some(name) = name_entry.file_name().to_str().map(str::to_owned) else {
            continue;
        };
        let Ok(versions) = std::fs::read_dir(&name_dir) else {
            continue;
        };
        for ver_entry in versions.flatten() {
            if !ver_entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                continue;
            }
            let ver_dir = ver_entry.path();
            let Some(version) = ver_entry.file_name().to_str().map(str::to_owned) else {
                continue;
            };
            match load_manifest(&ver_dir) {
                Ok(manifest) => out.push(InstalledTemplate {
                    name: manifest.name.clone().unwrap_or_else(|| name.clone()),
                    version: manifest.version.clone().unwrap_or_else(|| version.clone()),
                    path: ver_dir,
                    manifest,
                }),
                Err(reason) => {
                    tracing::warn!(
                        template = name.as_str(),
                        version = version.as_str(),
                        reason = %reason,
                        "skipping template without valid manifest"
                    );
                }
            }
        }
    }
    out.sort_by(|a, b| a.name.cmp(&b.name).then_with(|| version_cmp(&b.version, &a.version)));
    out
}

/// Locates a single installed template by name (and optional version).
///
/// When `version` is `None`, picks the highest-sorting version (numeric
/// segments compared as integers, falling back to lex order).
pub fn find_template(
    name: &str,
    version: Option<&str>,
) -> Result<InstalledTemplate, ErrorEnvelope> {
    let root = global_templates_root().ok_or_else(|| missing_root_error())?;
    find_template_in(&root, name, version)
}

/// Rooted variant of [`find_template`] for tests.
pub fn find_template_in(
    root: &Path,
    name: &str,
    version: Option<&str>,
) -> Result<InstalledTemplate, ErrorEnvelope> {
    let mut candidates: Vec<InstalledTemplate> = list_installed_in(root)
        .into_iter()
        .filter(|t| t.name == name)
        .collect();
    if candidates.is_empty() {
        return Err(not_found_error(name, version));
    }
    match version {
        Some(v) => candidates
            .into_iter()
            .find(|t| t.version == v)
            .ok_or_else(|| not_found_error(name, Some(v))),
        None => {
            candidates.sort_by(|a, b| version_cmp(&b.version, &a.version));
            candidates
                .into_iter()
                .next()
                .ok_or_else(|| not_found_error(name, None))
        }
    }
}

pub(crate) fn load_manifest(version_dir: &Path) -> Result<TemplateManifest, String> {
    let path = version_dir.join(MANIFEST_FILENAME);
    let raw = std::fs::read_to_string(&path)
        .map_err(|e| format!("read {}: {e}", path.display()))?;
    toml::from_str::<TemplateManifest>(&raw).map_err(|e| format!("parse: {e}"))
}

/// Component-wise version comparison: numeric segments compared as integers,
/// non-numeric segments compared lexicographically. `pub(crate)` so the
/// installer can pick the highest version directory from a freshly extracted
/// tarball without duplicating the logic.
pub(crate) fn version_cmp(a: &str, b: &str) -> Ordering {
    let mut ai = a.split('.');
    let mut bi = b.split('.');
    loop {
        match (ai.next(), bi.next()) {
            (None, None) => return Ordering::Equal,
            (None, Some(_)) => return Ordering::Less,
            (Some(_), None) => return Ordering::Greater,
            (Some(x), Some(y)) => {
                let ord = match (x.parse::<u64>(), y.parse::<u64>()) {
                    (Ok(xn), Ok(yn)) => xn.cmp(&yn),
                    _ => x.cmp(y),
                };
                if ord != Ordering::Equal {
                    return ord;
                }
            }
        }
    }
}

fn missing_root_error() -> ErrorEnvelope {
    ErrorEnvelope {
        kind: Kind::ConfigError,
        msg: "cannot resolve ~/.crud/templates (home directory unavailable)".into(),
        exit_code: Kind::ConfigError.exit_code(),
        hint: String::new(),
        details: serde_json::Map::new(),
    }
}

fn not_found_error(name: &str, version: Option<&str>) -> ErrorEnvelope {
    let detail = match version {
        Some(v) => format!("{name}@{v}"),
        None => name.to_owned(),
    };
    let mut details = serde_json::Map::new();
    details.insert("template".into(), serde_json::Value::String(detail.clone()));
    ErrorEnvelope {
        kind: Kind::UserError,
        msg: format!("template not installed: {detail}"),
        exit_code: Kind::UserError.exit_code(),
        hint: String::new(),
        details,
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]
    use super::*;
    use tempfile::TempDir;

    fn write_manifest(dir: &Path, body: &str) {
        std::fs::create_dir_all(dir).expect("mkdir");
        std::fs::write(dir.join(MANIFEST_FILENAME), body).expect("write");
    }

    #[test]
    fn version_sort_numeric() {
        assert_eq!(version_cmp("1.10.0", "1.2.0"), Ordering::Greater);
        assert_eq!(version_cmp("0.9.0", "0.10.0"), Ordering::Less);
        assert_eq!(version_cmp("1.0", "1.0.0"), Ordering::Less);
    }

    #[test]
    fn lists_templates_skipping_missing_manifest() {
        let tmp = TempDir::new().expect("tmp");
        let root = tmp.path();
        write_manifest(
            &root.join("ruoyi").join("1.0.0"),
            "backend = \"java\"\nfrontend = \"vue\"\n",
        );
        write_manifest(
            &root.join("ruoyi").join("1.1.0"),
            "backend = \"java\"\nfrontend = \"vue\"\ndescription = \"newer\"\n",
        );
        std::fs::create_dir_all(root.join("broken").join("0.1.0")).expect("mkdir");

        let list = list_installed_in(root);
        assert_eq!(list.len(), 2);
        assert!(list.iter().all(|t| t.name == "ruoyi"));
        assert_eq!(list[0].version, "1.1.0");
    }

    #[test]
    fn find_template_picks_highest_version_by_default() {
        let tmp = TempDir::new().expect("tmp");
        let root = tmp.path();
        write_manifest(
            &root.join("eladmin").join("0.9.0"),
            "backend = \"go\"\nfrontend = \"react\"\n",
        );
        write_manifest(
            &root.join("eladmin").join("0.10.0"),
            "backend = \"go\"\nfrontend = \"react\"\n",
        );
        let t = find_template_in(root, "eladmin", None).expect("found");
        assert_eq!(t.version, "0.10.0");
    }

    #[test]
    fn find_template_exact_version() {
        let tmp = TempDir::new().expect("tmp");
        let root = tmp.path();
        write_manifest(
            &root.join("nest-admin").join("2.0.0"),
            "backend = \"typescript\"\nfrontend = \"react\"\n",
        );
        let t = find_template_in(root, "nest-admin", Some("2.0.0")).expect("found");
        assert_eq!(t.version, "2.0.0");

        let err = find_template_in(root, "nest-admin", Some("9.9.9")).expect_err("missing");
        assert_eq!(err.kind, Kind::UserError);
    }

    #[test]
    fn find_template_custom_language_accepted() {
        let tmp = TempDir::new().expect("tmp");
        let root = tmp.path();
        write_manifest(
            &root.join("laravel-vue").join("1.0.0"),
            "backend = \"php\"\nfrontend = \"vue\"\n",
        );
        let t = find_template_in(root, "laravel-vue", None).expect("found");
        assert!(matches!(t.manifest.backend, Backend::Custom(ref s) if s == "php"));
    }
}
