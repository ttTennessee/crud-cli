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

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use flate2::read::GzDecoder;

use super::error::{ErrorEnvelope, Kind};
use super::template_meta_global::{
    load_manifest, version_cmp, InstalledTemplate, MANIFEST_FILENAME,
};

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

/// Install `<name>[@<version>]` from `spec` into `dest_root/<name>/<version>/`.
///
/// When `version` is `None`, picks the highest-sorting version directory
/// (matching the precedence used by `template_meta_global::find_template`).
pub fn install_template(
    name: &str,
    version: Option<&str>,
    spec: &RepoSpec,
    dest_root: &Path,
    force: bool,
) -> Result<InstalledTemplate, ErrorEnvelope> {
    let tmp = tempfile::tempdir().map_err(|e| io_error("create tempdir", e))?;
    let extract_dir = tmp.path().join("extracted");
    fs::create_dir_all(&extract_dir).map_err(|e| io_error("create extract dir", e))?;

    let url = spec.tarball_url();
    download_and_extract(&url, &extract_dir)?;

    let root_dir = single_child_dir(&extract_dir).ok_or_else(|| {
        network_error(format!("tarball from {url} has unexpected layout"))
    })?;
    let template_dir = root_dir.join(name);
    if !template_dir.is_dir() {
        return Err(template_not_in_repo(name, spec));
    }

    let src_version = match version {
        Some(v) => {
            let p = template_dir.join(v);
            if !p.join(MANIFEST_FILENAME).is_file() {
                return Err(version_not_in_repo(name, v, spec));
            }
            v.to_string()
        }
        None => pick_highest_version(&template_dir)
            .ok_or_else(|| no_version_in_repo(name, spec))?,
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
    let parent = dest_dir
        .parent()
        .expect("dest_root/<name> always has a parent");
    fs::create_dir_all(parent)
        .map_err(|e| io_error(format!("create {}", parent.display()), e))?;
    copy_dir_all(&src, &dest_dir)
        .map_err(|e| io_error(format!("copy into {}", dest_dir.display()), e))?;

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
