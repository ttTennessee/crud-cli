//! Download templates from a remote GitHub repository and install them under
//! `~/.crud/templates/<name>/<version>/`.
//!
//! Repo layout expected by this installer:
//!
//! ```text
//! <repo-root>/
//!   <name>/
//!     <version>/
//!       template.toml
//!       <bundles…>
//! ```
//!
//! The tarball is fetched via `codeload.github.com` (no auth, no git client
//! dependency) and unpacked into a tempdir; only the requested
//! `<name>/<version>/` subtree is copied into place.

use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use flate2::read::GzDecoder;

use super::error::{ErrorEnvelope, Kind};
use super::template_install_meta::{
    hash_dir, write_install_meta, InstallMeta, INSTALL_META_FILENAME,
};
use super::template_meta_global::{
    load_manifest, version_cmp, InstalledTemplate, MANIFEST_FILENAME,
};

/// Top-level directory in the templates repo that holds API-doc bundles
/// shared across every template. Per-template `<name>/<version>/doc/`
/// overrides it: when present, the global directory is not copied.
pub const SHARED_DOC_DIR: &str = "doc";

/// Top-level repo directories holding shared "pick-one" bundles that can be
/// layered onto a template at install time. Each is organised as
/// `<kind>/<category>/*.hbs` (e.g. `doc/html/…`, `sql/mysql/…`). A template
/// that ships its own `<kind>/` overrides the shared bundle for that kind, and
/// these names never appear as template names in [`RepoSnapshot::catalog`].
/// Keep in sync with `template_install_meta::SHARED_BUNDLE_DIRNAMES`.
pub const SHARED_BUNDLE_KINDS: &[&str] = &["doc", "sql"];

/// Git ref used when callers don't specify one (codeload accepts `HEAD`).
pub const DEFAULT_GIT_REF: &str = "HEAD";

/// Parsed remote repository spec.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepoSpec {
    pub owner: String,
    pub repo: String,
    pub git_ref: String,
}

impl RepoSpec {
    /// Accepts `owner/repo`, `owner/repo@ref`, `https://github.com/owner/repo`,
    /// and `https://github.com/owner/repo.git`. `@ref` is only recognized in
    /// the bare `owner/repo` form (full URLs always default to `HEAD`).
    pub fn parse(input: &str) -> Result<Self, String> {
        let trimmed = input.trim();
        if trimmed.is_empty() {
            return Err("repo spec must not be empty".into());
        }

        let is_url =
            trimmed.starts_with("https://") || trimmed.starts_with("http://");
        let (base, git_ref) = if is_url {
            (trimmed, DEFAULT_GIT_REF.to_string())
        } else if let Some((b, r)) = trimmed.rsplit_once('@') {
            if r.is_empty() {
                return Err("empty git ref after '@'".into());
            }
            (b, r.to_string())
        } else {
            (trimmed, DEFAULT_GIT_REF.to_string())
        };

        let stripped = base
            .strip_prefix("https://github.com/")
            .or_else(|| base.strip_prefix("http://github.com/"))
            .unwrap_or(base);
        let stripped = stripped.trim_end_matches('/');
        let stripped = stripped.strip_suffix(".git").unwrap_or(stripped);

        let (owner, repo) = stripped
            .split_once('/')
            .ok_or_else(|| format!("expected owner/repo, got {input:?}"))?;
        if repo.contains('/') {
            return Err(format!("expected owner/repo, got {input:?}"));
        }
        if owner.is_empty() || repo.is_empty() {
            return Err(format!("invalid repo spec: {input:?}"));
        }
        Ok(Self {
            owner: owner.to_string(),
            repo: repo.to_string(),
            git_ref,
        })
    }

    /// `https://codeload.github.com/{owner}/{repo}/tar.gz/{ref}`.
    #[must_use]
    pub fn tarball_url(&self) -> String {
        format!(
            "https://codeload.github.com/{}/{}/tar.gz/{}",
            self.owner, self.repo, self.git_ref
        )
    }

    /// Human-friendly form, e.g. `ttTennessee/crud-templates@HEAD`.
    #[must_use]
    pub fn display(&self) -> String {
        format!("{}/{}@{}", self.owner, self.repo, self.git_ref)
    }
}

/// A downloaded + unpacked tarball, ready for inspection and install.
///
/// Holds the `TempDir` so the extracted tree stays alive until the snapshot
/// is dropped.
pub struct RepoSnapshot {
    _tmp: tempfile::TempDir,
    root: PathBuf,
    spec: RepoSpec,
}

impl RepoSnapshot {
    /// Downloads `spec.tarball_url()` and unpacks it into a tempdir.
    pub fn fetch(spec: RepoSpec) -> Result<Self, ErrorEnvelope> {
        let tmp = tempfile::tempdir().map_err(|e| io_error("create tempdir", e))?;
        let extract_dir = tmp.path().join("extracted");
        fs::create_dir_all(&extract_dir).map_err(|e| io_error("create extract dir", e))?;
        let url = spec.tarball_url();
        download_and_extract(&url, &extract_dir)?;
        let root = single_child_dir(&extract_dir).ok_or_else(|| {
            network_error(format!("tarball from {url} has unexpected layout"))
        })?;
        Ok(Self {
            _tmp: tmp,
            root,
            spec,
        })
    }

    /// Repo source the snapshot came from.
    #[must_use]
    pub fn spec(&self) -> &RepoSpec {
        &self.spec
    }

    /// Enumerates every `<name>/<version>/template.toml` under the snapshot,
    /// grouped by name and sorted by descending version.
    #[must_use]
    pub fn catalog(&self) -> BTreeMap<String, Vec<String>> {
        let mut out: BTreeMap<String, Vec<String>> = BTreeMap::new();
        let Ok(entries) = fs::read_dir(&self.root) else {
            return out;
        };
        for entry in entries.flatten() {
            if !entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                continue;
            }
            let Some(name) = entry.file_name().to_str().map(str::to_owned) else {
                continue;
            };
            if SHARED_BUNDLE_KINDS.contains(&name.as_str()) {
                continue;
            }
            let mut versions: Vec<String> = fs::read_dir(entry.path())
                .ok()
                .into_iter()
                .flatten()
                .flatten()
                .filter(|e| {
                    e.file_type().map(|t| t.is_dir()).unwrap_or(false)
                        && e.path().join(MANIFEST_FILENAME).is_file()
                })
                .filter_map(|e| e.file_name().to_str().map(str::to_owned))
                .collect();
            if versions.is_empty() {
                continue;
            }
            versions.sort_by(|a, b| version_cmp(b, a));
            out.insert(name, versions);
        }
        out
    }

    /// Absolute path to a `<name>/<version>/` directory inside the snapshot,
    /// if one exists with a parseable `template.toml`.
    #[must_use]
    pub fn template_dir(&self, name: &str, version: &str) -> Option<PathBuf> {
        let p = self.root.join(name).join(version);
        if p.join(MANIFEST_FILENAME).is_file() {
            Some(p)
        } else {
            None
        }
    }

    /// `true` when the template author ships their own
    /// `<name>/<version>/<kind>/` — in that case the shared bundle for `kind`
    /// (e.g. `doc`, `sql`) is ignored entirely.
    #[must_use]
    pub fn template_has_bundle(&self, name: &str, version: &str, kind: &str) -> bool {
        self.root.join(name).join(version).join(kind).is_dir()
    }

    /// Subdirectories of the repo-level `<kind>/` (e.g. doc → `["html",
    /// "markdown"]`, sql → `["mysql", "postgres"]`), sorted. Empty when the
    /// snapshot has no such shared bundle or it contains only files. Bundles
    /// are organised as `<kind>/<category>/*.hbs`; loose files at
    /// `<kind>/*.hbs` are ignored.
    #[must_use]
    pub fn shared_categories(&self, kind: &str) -> Vec<String> {
        let base = self.root.join(kind);
        let Ok(entries) = fs::read_dir(&base) else {
            return Vec::new();
        };
        let mut out: Vec<String> = entries
            .flatten()
            .filter(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false))
            .filter_map(|e| e.file_name().to_str().map(str::to_owned))
            .collect();
        out.sort();
        out
    }

    /// Copies one subdirectory of the snapshot's shared `<kind>/` into
    /// `<dest_version_dir>/<kind>/<category>/`. The bundle picker is
    /// single-select, so exactly one category is layered per kind. An existing
    /// destination dir for that category is replaced.
    pub fn copy_shared_category(
        &self,
        dest_version_dir: &Path,
        kind: &str,
        category: &str,
    ) -> Result<(), ErrorEnvelope> {
        let src = self.root.join(kind).join(category);
        if !src.is_dir() {
            return Err(ErrorEnvelope::user_error(
                format!("{kind} category not in repo: {category}"),
                None,
                Some(category),
                "",
            ));
        }
        let dst = dest_version_dir.join(kind).join(category);
        if let Some(parent) = dst.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| io_error(format!("create {}", parent.display()), e))?;
        }
        if dst.exists() {
            fs::remove_dir_all(&dst)
                .map_err(|e| io_error(format!("remove {}", dst.display()), e))?;
        }
        copy_dir_all(&src, &dst).map_err(|e| io_error(format!("copy {kind}/{category}"), e))?;
        Ok(())
    }
}

/// Install `<name>[@<version>]` from a `RepoSnapshot` into
/// `dest_root/<name>/<version>/`.
///
/// When `version` is `None`, picks the highest-sorting version directory
/// (matching the precedence used by `template_meta_global::find_template`).
///
/// Writes `<dest>/.install.json` with the source hash + repo provenance so
/// `template install`'s next-run picker can label the version. Shared bundle
/// layering (doc, sql) is handled separately by the caller via
/// [`RepoSnapshot::copy_shared_category`].
pub fn install_from_snapshot(
    snapshot: &RepoSnapshot,
    name: &str,
    version: Option<&str>,
    dest_root: &Path,
    force: bool,
) -> Result<InstalledTemplate, ErrorEnvelope> {
    let template_dir = snapshot.root.join(name);
    if !template_dir.is_dir() {
        return Err(template_not_in_repo(name, &snapshot.spec));
    }

    let src_version = match version {
        Some(v) => {
            let p = template_dir.join(v);
            if !p.join(MANIFEST_FILENAME).is_file() {
                return Err(version_not_in_repo(name, v, &snapshot.spec));
            }
            v.to_string()
        }
        None => pick_highest_version(&template_dir)
            .ok_or_else(|| no_version_in_repo(name, &snapshot.spec))?,
    };
    let src = template_dir.join(&src_version);

    // Validate manifest BEFORE writing anything; resolve effective name/version
    // through manifest fields (matches `template list` / `template use`).
    let manifest = load_manifest(&src).map_err(|reason| {
        manifest_unreadable(name, &src_version, &src, &reason)
    })?;
    let installed_name = manifest.name.clone().unwrap_or_else(|| name.to_string());
    let installed_version = manifest.version.clone().unwrap_or_else(|| src_version.clone());

    let dest_dir = dest_root.join(&installed_name).join(&installed_version);
    if dest_dir.exists() {
        if !force {
            return Err(already_installed(
                &installed_name,
                &installed_version,
                &dest_dir,
            ));
        }
        fs::remove_dir_all(&dest_dir)
            .map_err(|e| io_error(format!("remove {}", dest_dir.display()), e))?;
    }
    let parent = dest_dir.parent().ok_or_else(|| {
        io_error(
            format!("invalid dest path {}", dest_dir.display()),
            io::Error::new(io::ErrorKind::InvalidInput, "no parent directory"),
        )
    })?;
    fs::create_dir_all(parent)
        .map_err(|e| io_error(format!("create {}", parent.display()), e))?;
    copy_dir_all(&src, &dest_dir)
        .map_err(|e| io_error(format!("copy into {}", dest_dir.display()), e))?;

    // Pin provenance + content hash so the next `template install` can label
    // this version as "已安装" / "已修改" / "有新版本" without trusting mtimes.
    let source_hash = hash_dir(&dest_dir)
        .map_err(|e| io_error(format!("hash {}", dest_dir.display()), e))?;
    let meta = InstallMeta {
        source_hash,
        repo: format!("{}/{}", snapshot.spec.owner, snapshot.spec.repo),
        repo_ref: snapshot.spec.git_ref.clone(),
        installed_at: now_rfc3339(),
        doc_categories: Vec::new(),
        sql_categories: Vec::new(),
    };
    write_install_meta(&dest_dir, &meta)
        .map_err(|e| io_error(format!("write {INSTALL_META_FILENAME}"), e))?;

    Ok(InstalledTemplate {
        name: installed_name,
        version: installed_version,
        path: dest_dir,
        manifest,
    })
}

fn download_and_extract(url: &str, dest: &Path) -> Result<(), ErrorEnvelope> {
    let mut response = ureq::get(url)
        .call()
        .map_err(|e| network_error(format!("GET {url}: {e}")))?;
    if response.status().as_u16() >= 400 {
        return Err(network_error(format!(
            "GET {url}: status {}",
            response.status()
        )));
    }
    let reader = response.body_mut().as_reader();
    let gz = GzDecoder::new(reader);
    let mut archive = tar::Archive::new(gz);
    archive
        .unpack(dest)
        .map_err(|e| network_error(format!("unpack tarball: {e}")))?;
    Ok(())
}

fn single_child_dir(parent: &Path) -> Option<PathBuf> {
    let mut iter = fs::read_dir(parent).ok()?;
    let first = iter.next()?.ok()?.path();
    if iter.next().is_some() {
        return None;
    }
    if first.is_dir() { Some(first) } else { None }
}

fn pick_highest_version(template_dir: &Path) -> Option<String> {
    let mut versions: Vec<String> = fs::read_dir(template_dir)
        .ok()?
        .flatten()
        .filter(|e| {
            e.file_type().map(|t| t.is_dir()).unwrap_or(false)
                && e.path().join(MANIFEST_FILENAME).is_file()
        })
        .filter_map(|e| e.file_name().to_str().map(str::to_owned))
        .collect();
    if versions.is_empty() {
        return None;
    }
    versions.sort_by(|a, b| version_cmp(b, a));
    versions.into_iter().next()
}

fn copy_dir_all(src: &Path, dst: &Path) -> io::Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ft = entry.file_type()?;
        let target = dst.join(entry.file_name());
        if ft.is_dir() {
            copy_dir_all(&entry.path(), &target)?;
        } else if ft.is_file() {
            fs::copy(entry.path(), &target)?;
        }
        // skip symlinks; templates shouldn't ship any.
    }
    Ok(())
}

fn now_rfc3339() -> String {
    time::OffsetDateTime::now_utc()
        .format(&time::format_description::well_known::Rfc3339)
        .unwrap_or_else(|_| "1970-01-01T00:00:00Z".into())
}

fn io_error(ctx: impl Into<String>, e: io::Error) -> ErrorEnvelope {
    ErrorEnvelope {
        kind: Kind::ConfigError,
        msg: format!("{}: {e}", ctx.into()),
        exit_code: Kind::ConfigError.exit_code(),
        hint: String::new(),
        details: serde_json::Map::new(),
    }
}

fn network_error(msg: impl Into<String>) -> ErrorEnvelope {
    ErrorEnvelope {
        kind: Kind::NetworkError,
        msg: msg.into(),
        exit_code: Kind::NetworkError.exit_code(),
        hint: String::new(),
        details: serde_json::Map::new(),
    }
}

fn template_not_in_repo(name: &str, spec: &RepoSpec) -> ErrorEnvelope {
    let mut details = serde_json::Map::new();
    details.insert("template".into(), serde_json::Value::String(name.into()));
    details.insert("repo".into(), serde_json::Value::String(spec.display()));
    ErrorEnvelope {
        kind: Kind::UserError,
        msg: format!("template not found in repo: {name}"),
        exit_code: Kind::UserError.exit_code(),
        hint: String::new(),
        details,
    }
}

fn version_not_in_repo(name: &str, version: &str, spec: &RepoSpec) -> ErrorEnvelope {
    let mut details = serde_json::Map::new();
    details.insert(
        "template".into(),
        serde_json::Value::String(format!("{name}@{version}")),
    );
    details.insert("repo".into(), serde_json::Value::String(spec.display()));
    ErrorEnvelope {
        kind: Kind::UserError,
        msg: format!("version not found in repo: {name}@{version}"),
        exit_code: Kind::UserError.exit_code(),
        hint: String::new(),
        details,
    }
}

fn no_version_in_repo(name: &str, spec: &RepoSpec) -> ErrorEnvelope {
    let mut details = serde_json::Map::new();
    details.insert("template".into(), serde_json::Value::String(name.into()));
    details.insert("repo".into(), serde_json::Value::String(spec.display()));
    ErrorEnvelope {
        kind: Kind::UserError,
        msg: format!("no installable versions for {name} in repo"),
        exit_code: Kind::UserError.exit_code(),
        hint: String::new(),
        details,
    }
}

fn already_installed(name: &str, version: &str, dest: &Path) -> ErrorEnvelope {
    let mut details = serde_json::Map::new();
    details.insert(
        "template".into(),
        serde_json::Value::String(format!("{name}@{version}")),
    );
    details.insert(
        "path".into(),
        serde_json::Value::String(dest.display().to_string()),
    );
    ErrorEnvelope {
        kind: Kind::FileConflict,
        msg: format!("{name}@{version} already installed at {}", dest.display()),
        exit_code: Kind::FileConflict.exit_code(),
        hint: String::new(),
        details,
    }
}

fn manifest_unreadable(
    name: &str,
    version: &str,
    src: &Path,
    reason: &str,
) -> ErrorEnvelope {
    let mut details = serde_json::Map::new();
    details.insert(
        "template".into(),
        serde_json::Value::String(format!("{name}@{version}")),
    );
    details.insert(
        "path".into(),
        serde_json::Value::String(src.display().to_string()),
    );
    details.insert("reason".into(), serde_json::Value::String(reason.into()));
    ErrorEnvelope {
        kind: Kind::ConfigError,
        msg: format!("{name}@{version} has invalid template.toml: {reason}"),
        exit_code: Kind::ConfigError.exit_code(),
        hint: String::new(),
        details,
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]
    use super::*;

    #[test]
    fn parses_owner_repo() {
        let s = RepoSpec::parse("ttTennessee/crud-templates").expect("ok");
        assert_eq!(s.owner, "ttTennessee");
        assert_eq!(s.repo, "crud-templates");
        assert_eq!(s.git_ref, "HEAD");
        assert_eq!(
            s.tarball_url(),
            "https://codeload.github.com/ttTennessee/crud-templates/tar.gz/HEAD"
        );
    }

    #[test]
    fn parses_owner_repo_with_ref() {
        let s = RepoSpec::parse("ttTennessee/crud-templates@main").expect("ok");
        assert_eq!(s.git_ref, "main");
    }

    #[test]
    fn parses_github_url() {
        let s = RepoSpec::parse("https://github.com/ttTennessee/crud-templates").expect("ok");
        assert_eq!(s.owner, "ttTennessee");
        assert_eq!(s.repo, "crud-templates");
        assert_eq!(s.git_ref, "HEAD");
    }

    #[test]
    fn parses_github_url_with_dot_git() {
        let s = RepoSpec::parse("https://github.com/foo/bar.git").expect("ok");
        assert_eq!(s.repo, "bar");
    }

    #[test]
    fn rejects_empty_and_garbage() {
        assert!(RepoSpec::parse("").is_err());
        assert!(RepoSpec::parse("justone").is_err());
        assert!(RepoSpec::parse("a/b/c").is_err());
        assert!(RepoSpec::parse("foo/bar@").is_err());
    }

    fn write_manifest(dir: &Path, body: &str) {
        fs::create_dir_all(dir).expect("mkdir");
        fs::write(dir.join(MANIFEST_FILENAME), body).expect("write");
    }

    fn snapshot_from(root: PathBuf) -> RepoSnapshot {
        // Construct without going through fetch(); we only need .root + .spec
        // for catalog/install tests.
        let tmp = tempfile::tempdir().expect("tmp");
        RepoSnapshot {
            _tmp: tmp,
            root,
            spec: RepoSpec::parse("ttTennessee/crud-templates").expect("ok"),
        }
    }

    #[test]
    fn catalog_groups_versions_and_filters_shared_doc() {
        let tmp = tempfile::tempdir().expect("tmp");
        let root = tmp.path().to_path_buf();
        write_manifest(
            &root.join("ruoyi").join("1.0.0"),
            "backend = \"java\"\nfrontend = \"vue\"\n",
        );
        write_manifest(
            &root.join("ruoyi").join("1.10.0"),
            "backend = \"java\"\nfrontend = \"vue\"\n",
        );
        write_manifest(
            &root.join("eladmin").join("2.0.0"),
            "backend = \"java\"\nfrontend = \"vue\"\n",
        );
        // shared doc/ at repo root must NOT appear as a template name.
        fs::create_dir_all(root.join("doc")).expect("mkdir");
        fs::write(root.join("doc").join("controller.md.hbs"), "x").expect("write");

        let snap = snapshot_from(root);
        let cat = snap.catalog();
        assert_eq!(cat.len(), 2);
        assert_eq!(cat["ruoyi"], vec!["1.10.0", "1.0.0"]);
        assert_eq!(cat["eladmin"], vec!["2.0.0"]);
        assert!(!cat.contains_key("doc"));
    }

    #[test]
    fn install_writes_install_meta_sidecar() {
        let tmp = tempfile::tempdir().expect("tmp");
        let root = tmp.path().join("repo");
        write_manifest(
            &root.join("ruoyi").join("1.0.0"),
            "backend = \"java\"\nfrontend = \"vue\"\n",
        );
        fs::write(
            root.join("ruoyi").join("1.0.0").join("Controller.java.hbs"),
            "",
        )
        .expect("write");

        let snap = snapshot_from(root);
        let dest = tmp.path().join("home");
        let installed =
            install_from_snapshot(&snap, "ruoyi", None, &dest, false).expect("install");

        let meta = crate::core::template_install_meta::load_install_meta(&installed.path)
            .expect("sidecar written");
        assert_eq!(meta.source_hash.len(), 64);
        assert!(meta.repo.ends_with("/crud-templates"));
        assert!(meta.doc_categories.is_empty());
    }

    #[test]
    fn shared_categories_lists_top_level_subdirs() {
        let tmp = tempfile::tempdir().expect("tmp");
        let root = tmp.path().join("repo");
        fs::create_dir_all(root.join("doc").join("markdown")).expect("mkdir");
        fs::create_dir_all(root.join("doc").join("html")).expect("mkdir");
        fs::write(root.join("doc").join("README.md"), "x").expect("write");
        fs::write(root.join("doc").join("markdown").join("a.hbs"), "a").expect("write");
        fs::create_dir_all(root.join("sql").join("mysql")).expect("mkdir");
        fs::create_dir_all(root.join("sql").join("postgres")).expect("mkdir");

        let snap = snapshot_from(root);
        assert_eq!(
            snap.shared_categories("doc"),
            vec!["html".to_string(), "markdown".to_string()]
        );
        assert_eq!(
            snap.shared_categories("sql"),
            vec!["mysql".to_string(), "postgres".to_string()]
        );
        assert!(snap.shared_categories("missing").is_empty());
    }

    #[test]
    fn catalog_filters_sql_bundle_dir() {
        let tmp = tempfile::tempdir().expect("tmp");
        let root = tmp.path().to_path_buf();
        write_manifest(
            &root.join("ruoyi").join("1.0.0"),
            "backend = \"java\"\nfrontend = \"vue\"\n",
        );
        fs::create_dir_all(root.join("sql").join("mysql")).expect("mkdir");
        fs::write(root.join("sql").join("mysql").join("schema.sql.hbs"), "x").expect("write");

        let snap = snapshot_from(root);
        let cat = snap.catalog();
        assert!(cat.contains_key("ruoyi"));
        assert!(!cat.contains_key("sql"));
    }

    #[test]
    fn copy_shared_category_copies_only_the_picked_one() {
        let tmp = tempfile::tempdir().expect("tmp");
        let root = tmp.path().join("repo");
        write_manifest(
            &root.join("ruoyi").join("1.0.0"),
            "backend = \"java\"\nfrontend = \"vue\"\n",
        );
        fs::create_dir_all(root.join("sql").join("mysql")).expect("mkdir");
        fs::write(root.join("sql").join("mysql").join("a.sql.hbs"), "a").expect("write");
        fs::create_dir_all(root.join("sql").join("postgres")).expect("mkdir");
        fs::write(root.join("sql").join("postgres").join("b.sql.hbs"), "b").expect("write");

        let snap = snapshot_from(root);
        let dest = tmp.path().join("home");
        let installed =
            install_from_snapshot(&snap, "ruoyi", None, &dest, false).expect("install");

        snap.copy_shared_category(&installed.path, "sql", "mysql")
            .expect("copy");
        assert!(installed.path.join("sql").join("mysql").join("a.sql.hbs").is_file());
        assert!(!installed.path.join("sql").join("postgres").exists());
    }

    #[test]
    fn copy_shared_category_rejects_missing_category() {
        let tmp = tempfile::tempdir().expect("tmp");
        let root = tmp.path().join("repo");
        write_manifest(
            &root.join("ruoyi").join("1.0.0"),
            "backend = \"java\"\nfrontend = \"vue\"\n",
        );
        let snap = snapshot_from(root);
        let dest = tmp.path().join("home");
        let installed =
            install_from_snapshot(&snap, "ruoyi", None, &dest, false).expect("install");
        assert!(snap
            .copy_shared_category(&installed.path, "sql", "nope")
            .is_err());
    }

    #[test]
    fn template_has_bundle_detects_bundled_dir() {
        let tmp = tempfile::tempdir().expect("tmp");
        let root = tmp.path().join("repo");
        write_manifest(
            &root.join("ruoyi").join("1.0.0"),
            "backend = \"java\"\nfrontend = \"vue\"\n",
        );
        fs::create_dir_all(root.join("ruoyi").join("1.0.0").join("doc")).expect("mkdir");
        fs::create_dir_all(root.join("ruoyi").join("1.0.0").join("sql")).expect("mkdir");
        write_manifest(
            &root.join("eladmin").join("2.0.0"),
            "backend = \"java\"\nfrontend = \"vue\"\n",
        );

        let snap = snapshot_from(root);
        assert!(snap.template_has_bundle("ruoyi", "1.0.0", "doc"));
        assert!(snap.template_has_bundle("ruoyi", "1.0.0", "sql"));
        assert!(!snap.template_has_bundle("eladmin", "2.0.0", "doc"));
        assert!(!snap.template_has_bundle("eladmin", "2.0.0", "sql"));
    }

    #[test]
    fn pick_highest_version_picks_numeric_max() {
        let tmp = tempfile::tempdir().expect("tmp");
        let tdir = tmp.path().join("ruoyi");
        for v in ["1.0.0", "1.10.0", "1.2.0"] {
            let d = tdir.join(v);
            fs::create_dir_all(&d).expect("mkdir");
            fs::write(d.join(MANIFEST_FILENAME), "backend = \"java\"\nfrontend = \"vue\"\n")
                .expect("write");
        }
        // a stray version directory without manifest must be ignored.
        fs::create_dir_all(tdir.join("9.9.9-broken")).expect("mkdir broken");

        assert_eq!(pick_highest_version(&tdir).as_deref(), Some("1.10.0"));
    }
}
