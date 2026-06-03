//! Sidecar `.install.json` inside each installed `<version>/` directory.
//!
//! Records the SHA-256 of the template tree at install time plus the repo it
//! came from. `template install` reads this on the next run to label each
//! version in the version picker:
//!
//! * stored hash == on-disk hash == repo hash → 已安装
//! * stored hash != on-disk hash             → 已安装·已修改 (user edits)
//! * stored hash == on-disk hash, != repo    → 已安装·有新版本 (repo moved)
//!
//! The file is excluded from every hash computation so writing it does not
//! make the install look modified.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Filename of the sidecar under `<name>/<version>/`.
pub const INSTALL_META_FILENAME: &str = ".install.json";

/// Top-level directory names that hold layered "pick-one" bundles (doc, ddl, sql),
/// excluded from the template hash so layering a shared bundle after install
/// does not flip the modification flag. Keep in sync with
/// `template_installer::SHARED_BUNDLE_KINDS`.
const SHARED_BUNDLE_DIRNAMES: &[&str] = &["doc", "ddl", "sql"];

/// Persisted install metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallMeta {
    /// SHA-256 (hex) of the template tree at install time, excluding the
    /// sidecar itself.
    pub source_hash: String,
    /// `owner/repo` the template was installed from.
    pub repo: String,
    /// Git ref the snapshot was downloaded at (e.g. `HEAD`, `main`, a SHA).
    pub repo_ref: String,
    /// RFC-3339 UTC timestamp of the install.
    pub installed_at: String,
    /// Doc category layered on top of the template at install time
    /// (a subdirectory of the repo-level `doc/`). Empty when the template
    /// shipped its own `doc/` or the user picked nothing. Holds at most one
    /// entry since the doc picker is single-select.
    #[serde(default)]
    pub doc_categories: Vec<String>,
    /// DDL category layered at install time (subdirectory of repo-level `ddl/`,
    /// e.g. `mysql` for `schema.sql.hbs`). Same semantics as [`Self::doc_categories`].
    #[serde(default)]
    pub ddl_categories: Vec<String>,
    /// SQL category layered at install time (repo-level `sql/` when used for
    /// shared data SQL bundles). Per-template `sql/` (e.g. menu) ships with the
    /// template and does not use this field.
    #[serde(default)]
    pub sql_categories: Vec<String>,
}

/// `Some(meta)` if `<version>/.install.json` exists AND parses; `None`
/// otherwise. A corrupt sidecar is treated as missing so the picker degrades
/// to "未安装"-style behavior instead of crashing.
pub fn load_install_meta(version_dir: &Path) -> Option<InstallMeta> {
    let path = version_dir.join(INSTALL_META_FILENAME);
    let raw = fs::read_to_string(&path).ok()?;
    serde_json::from_str(&raw).ok()
}

/// Writes `<version>/.install.json`. Overwrites any existing sidecar.
pub fn write_install_meta(version_dir: &Path, meta: &InstallMeta) -> io::Result<()> {
    let path = version_dir.join(INSTALL_META_FILENAME);
    let body = serde_json::to_string_pretty(meta)
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
    fs::write(path, body)
}

/// Updates `<version>/.install.json`'s layered-category field for `kind`
/// (`"ddl"` → `ddl_categories`, `"sql"` → `sql_categories`, else `doc_categories`)
/// in place. Fails when no sidecar exists yet; only the bundle-copy step calls
/// this.
pub fn record_bundle_categories(
    version_dir: &Path,
    kind: &str,
    categories: &[String],
) -> io::Result<()> {
    let mut meta = load_install_meta(version_dir)
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "no .install.json"))?;
    match kind {
        "sql" => meta.sql_categories = categories.to_vec(),
        "ddl" => meta.ddl_categories = categories.to_vec(),
        _ => meta.doc_categories = categories.to_vec(),
    }
    write_install_meta(version_dir, &meta)
}

/// SHA-256 of `dir`'s recursive contents. Files are folded in sorted order of
/// their path relative to `dir`; each contribution is `len(rel) | rel | 0 |
/// len(bytes) | bytes | 0`. The top-level shared-bundle subtrees (`doc/`,
/// `sql/`) and `.install.json` at any depth are skipped so layering a shared
/// bundle and writing the sidecar itself do not change the hash. Symlinks are
/// skipped (templates don't ship any).
pub fn hash_dir(dir: &Path) -> io::Result<String> {
    let mut files: Vec<(PathBuf, Vec<u8>)> = Vec::new();
    collect(dir, dir, &mut files)?;
    files.sort_by(|a, b| a.0.cmp(&b.0));
    let mut hasher = Sha256::new();
    for (rel, body) in files {
        let rel_bytes = rel.to_string_lossy().into_owned().into_bytes();
        hasher.update((rel_bytes.len() as u64).to_le_bytes());
        hasher.update(&rel_bytes);
        hasher.update([0]);
        hasher.update((body.len() as u64).to_le_bytes());
        hasher.update(&body);
        hasher.update([0]);
    }
    Ok(hex(hasher.finalize().as_slice()))
}

fn collect(root: &Path, dir: &Path, out: &mut Vec<(PathBuf, Vec<u8>)>) -> io::Result<()> {
    let at_root = dir == root;
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let ft = entry.file_type()?;
        let name = entry.file_name();
        if name == INSTALL_META_FILENAME {
            continue;
        }
        if at_root
            && ft.is_dir()
            && name
                .to_str()
                .is_some_and(|n| SHARED_BUNDLE_DIRNAMES.contains(&n))
        {
            continue;
        }
        let path = entry.path();
        if ft.is_dir() {
            collect(root, &path, out)?;
        } else if ft.is_file() {
            let rel = path
                .strip_prefix(root)
                .map(Path::to_path_buf)
                .unwrap_or_else(|_| path.clone());
            let body = fs::read(&path)?;
            out.push((rel, body));
        }
    }
    Ok(())
}

fn hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        s.push(HEX[(b >> 4) as usize] as char);
        s.push(HEX[(b & 0x0f) as usize] as char);
    }
    s
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn hash_is_stable_across_calls() {
        let tmp = TempDir::new().expect("tmp");
        fs::write(tmp.path().join("a.txt"), "hello").expect("write");
        fs::create_dir_all(tmp.path().join("sub")).expect("mkdir");
        fs::write(tmp.path().join("sub").join("b.txt"), "world").expect("write");
        let h1 = hash_dir(tmp.path()).expect("h1");
        let h2 = hash_dir(tmp.path()).expect("h2");
        assert_eq!(h1, h2);
        assert_eq!(h1.len(), 64);
    }

    #[test]
    fn hash_changes_when_content_changes() {
        let tmp = TempDir::new().expect("tmp");
        fs::write(tmp.path().join("a.txt"), "hello").expect("w");
        let before = hash_dir(tmp.path()).expect("h");
        fs::write(tmp.path().join("a.txt"), "hello!").expect("w");
        let after = hash_dir(tmp.path()).expect("h");
        assert_ne!(before, after);
    }

    #[test]
    fn install_meta_is_excluded_from_hash() {
        let tmp = TempDir::new().expect("tmp");
        fs::write(tmp.path().join("a.txt"), "x").expect("w");
        let before = hash_dir(tmp.path()).expect("h");
        fs::write(tmp.path().join(INSTALL_META_FILENAME), "{}").expect("w");
        let after = hash_dir(tmp.path()).expect("h");
        assert_eq!(before, after);
    }

    #[test]
    fn top_level_doc_dir_is_excluded_from_hash() {
        let tmp = TempDir::new().expect("tmp");
        fs::write(tmp.path().join("a.txt"), "x").expect("w");
        let before = hash_dir(tmp.path()).expect("h");
        fs::create_dir_all(tmp.path().join("doc").join("markdown")).expect("mkdir");
        fs::write(tmp.path().join("doc").join("markdown").join("z.hbs"), "z").expect("w");
        let after = hash_dir(tmp.path()).expect("h");
        assert_eq!(before, after);
    }

    #[test]
    fn top_level_sql_dir_is_excluded_from_hash() {
        let tmp = TempDir::new().expect("tmp");
        fs::write(tmp.path().join("a.txt"), "x").expect("w");
        let before = hash_dir(tmp.path()).expect("h");
        fs::create_dir_all(tmp.path().join("sql").join("mysql")).expect("mkdir");
        fs::write(tmp.path().join("sql").join("mysql").join("z.hbs"), "z").expect("w");
        let after = hash_dir(tmp.path()).expect("h");
        assert_eq!(before, after);
    }

    #[test]
    fn nested_doc_dir_is_not_excluded() {
        // Only the TOP-LEVEL doc/ is special; a doc/ deeper in the tree is
        // ordinary template content.
        let tmp = TempDir::new().expect("tmp");
        fs::create_dir_all(tmp.path().join("src").join("doc")).expect("mkdir");
        fs::write(tmp.path().join("src").join("doc").join("a.hbs"), "a").expect("w");
        let before = hash_dir(tmp.path()).expect("h");
        fs::write(tmp.path().join("src").join("doc").join("a.hbs"), "b").expect("w");
        let after = hash_dir(tmp.path()).expect("h");
        assert_ne!(before, after);
    }

    #[test]
    fn hash_is_path_sensitive() {
        let tmp1 = TempDir::new().expect("t1");
        fs::write(tmp1.path().join("a.txt"), "x").expect("w");
        let tmp2 = TempDir::new().expect("t2");
        fs::write(tmp2.path().join("b.txt"), "x").expect("w");
        assert_ne!(
            hash_dir(tmp1.path()).expect("h1"),
            hash_dir(tmp2.path()).expect("h2")
        );
    }

    #[test]
    fn roundtrip_meta() {
        let tmp = TempDir::new().expect("tmp");
        let meta = InstallMeta {
            source_hash: "abc".into(),
            repo: "owner/repo".into(),
            repo_ref: "HEAD".into(),
            installed_at: "2026-05-29T00:00:00Z".into(),
            doc_categories: vec!["markdown".into()],
            ddl_categories: vec!["mysql".into()],
            sql_categories: Vec::new(),
        };
        write_install_meta(tmp.path(), &meta).expect("write");
        let back = load_install_meta(tmp.path()).expect("load");
        assert_eq!(back.source_hash, "abc");
        assert_eq!(back.doc_categories, vec!["markdown".to_string()]);
        assert_eq!(back.ddl_categories, vec!["mysql".to_string()]);
    }

    #[test]
    fn record_bundle_categories_routes_by_kind() {
        let tmp = TempDir::new().expect("tmp");
        let meta = InstallMeta {
            source_hash: "abc".into(),
            repo: "owner/repo".into(),
            repo_ref: "HEAD".into(),
            installed_at: "2026-05-29T00:00:00Z".into(),
            doc_categories: Vec::new(),
            ddl_categories: Vec::new(),
            sql_categories: Vec::new(),
        };
        write_install_meta(tmp.path(), &meta).expect("write");
        record_bundle_categories(tmp.path(), "doc", &["html".into()]).expect("doc");
        record_bundle_categories(tmp.path(), "ddl", &["mysql".into()]).expect("ddl");
        record_bundle_categories(tmp.path(), "sql", &["postgres".into()]).expect("sql");
        let back = load_install_meta(tmp.path()).expect("load");
        assert_eq!(back.doc_categories, vec!["html".to_string()]);
        assert_eq!(back.ddl_categories, vec!["mysql".to_string()]);
        assert_eq!(back.sql_categories, vec!["postgres".to_string()]);
    }

    #[test]
    fn load_returns_none_when_missing_or_corrupt() {
        let tmp = TempDir::new().expect("tmp");
        assert!(load_install_meta(tmp.path()).is_none());
        fs::write(tmp.path().join(INSTALL_META_FILENAME), "not json").expect("w");
        assert!(load_install_meta(tmp.path()).is_none());
    }
}
