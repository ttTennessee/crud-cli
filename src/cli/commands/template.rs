//! `crud-cli template ...` — manage installed templates under
//! `~/.crud/templates/`.

use std::env;
use std::io::IsTerminal;
use std::path::Path;

use inquire::{Confirm, MultiSelect, Select};

use crate::cli::agent_mode::is_agent_active;
use crate::cli::args::{exit_with_envelope, TemplateArgs, TemplateCommand};
use crate::cli::output::emit_success;
use crate::core::config::{load_setup_file, SetupConfig, TemplateRef};
use crate::core::error::{ErrorEnvelope, Kind};
use crate::core::global_config::GlobalConfig;
use crate::core::i18n::{self, keys};
use crate::core::paths::{global_config_toml, project_setup_toml};
use crate::core::template_install_meta::{hash_dir, load_install_meta, record_doc_categories};
use crate::core::template_installer::{install_from_snapshot, RepoSnapshot, RepoSpec};
use crate::core::template_meta_global::{
    find_template, global_templates_root, list_installed_templates, InstalledTemplate,
};

pub fn run_template(args: TemplateArgs) -> i32 {
    match args.command {
        TemplateCommand::List => match cmd_list() {
            Ok(()) => 0,
            Err(env) => exit_with_envelope(&env),
        },
        TemplateCommand::Use { name, yes } => match cmd_use(&name, yes) {
            Ok(()) => 0,
            Err(env) => exit_with_envelope(&env),
        },
        TemplateCommand::Install { name, repo, force } => {
            match cmd_install(name.as_deref(), repo.as_deref(), force) {
                Ok(()) => 0,
                Err(env) => exit_with_envelope(&env),
            }
        }
    }
}

fn cmd_install(
    spec: Option<&str>,
    repo_override: Option<&str>,
    force: bool,
) -> Result<(), ErrorEnvelope> {
    // 1. Resolve repo.
    let raw_repo: String = match repo_override {
        Some(s) => s.to_string(),
        None => {
            let cfg = global_config_toml()
                .map(|p| GlobalConfig::load_or_default(&p))
                .unwrap_or_default();
            cfg.template_repo().to_string()
        }
    };
    let repo = RepoSpec::parse(&raw_repo).map_err(|e| {
        ErrorEnvelope::user_error(format!("invalid --repo: {e}"), Some("repo"), Some(&raw_repo), "")
    })?;

    // 2. Parse the optional name@version. We need this BEFORE downloading so
    // that `install foo@bar` skips both pickers.
    let parsed = match spec {
        Some(s) => Some(TemplateRef::parse(s).map_err(|e| {
            ErrorEnvelope::user_error(
                format!("invalid template: {e}"),
                Some("template"),
                Some(s),
                "",
            )
        })?),
        None => None,
    };

    let dest_root = global_templates_root().ok_or_else(|| ErrorEnvelope {
        kind: Kind::ConfigError,
        msg: "cannot resolve ~/.crud/templates".into(),
        exit_code: Kind::ConfigError.exit_code(),
        hint: String::new(),
        details: serde_json::Map::new(),
    })?;

    // 3. Download + extract (single network round-trip; reused by any picker).
    let starting_target = parsed
        .as_ref()
        .map(|t| t.name.clone())
        .unwrap_or_else(|| i18n::t(keys::TEMPLATE_INSTALL_PROMPT_FETCH).to_string());
    let starting = i18n::tf(
        keys::TEMPLATE_INSTALL_STARTING,
        &[("name", &starting_target), ("repo", &repo.display())],
    );
    emit_success(Some(&starting));
    let snapshot = RepoSnapshot::fetch(repo)?;

    // 4. Resolve name + version, prompting only when not provided.
    // The version picker labels each entry with its on-disk status (installed
    // / locally-modified / repo-updated) and, when an installed version is
    // picked, asks to confirm a reinstall — effectively promoting `force`.
    let (name, version, picker_force) =
        resolve_name_and_version(&snapshot, parsed.as_ref(), &dest_root)?;
    let effective_force = force || picker_force;

    let installed = install_from_snapshot(
        &snapshot,
        &name,
        version.as_deref(),
        &dest_root,
        effective_force,
    )?;

    // 5. Doc picker — only when the template doesn't bundle its own doc/
    // (we never overwrite author-shipped doc) AND we're in an interactive
    // session. Non-TTY installs land with no doc bundles by design; users
    // can rerun `template install --force` interactively to add them.
    let doc_cats =
        pick_doc_categories(&snapshot, &installed.name, &installed.version, parsed.as_ref())?;
    if !doc_cats.is_empty() {
        snapshot.copy_shared_doc(&installed.path, &doc_cats)?;
        record_doc_categories(&installed.path, &doc_cats).map_err(|e| ErrorEnvelope {
            kind: Kind::ConfigError,
            msg: format!("record doc categories: {e}"),
            exit_code: Kind::ConfigError.exit_code(),
            hint: String::new(),
            details: serde_json::Map::new(),
        })?;
    }

    let done = i18n::tf(
        keys::TEMPLATE_INSTALL_DONE,
        &[
            ("name", &installed.name),
            ("version", &installed.version),
            ("backend", installed.manifest.backend.as_key()),
            ("frontend", installed.manifest.frontend.as_key()),
            ("path", &installed.path.display().to_string()),
        ],
    );
    emit_success(Some(&done));
    Ok(())
}

/// Resolves the install target, asking the user to pick whatever's missing.
///
/// Returns `(name, version, picker_force)`. `picker_force` is `true` when the
/// user picked an already-installed version from the picker and confirmed a
/// reinstall — semantically equivalent to passing `--force` on the CLI.
/// `(Some(name), Some(version))` short-circuits both pickers (and yields
/// `picker_force = false`; the caller's `--force` still applies).
fn resolve_name_and_version(
    snapshot: &RepoSnapshot,
    parsed: Option<&TemplateRef>,
    dest_root: &Path,
) -> Result<(String, Option<String>, bool), ErrorEnvelope> {
    let catalog = snapshot.catalog();
    if catalog.is_empty() {
        return Err(ErrorEnvelope::user_error(
            "no installable templates found in repo".to_string(),
            None,
            None,
            i18n::t(keys::ERROR_TEMPLATE_INSTALL_REPO_EMPTY),
        ));
    }

    let name = match parsed.map(|t| t.name.clone()) {
        Some(n) => n,
        None => {
            require_interactive("template name")?;
            let names: Vec<String> = catalog.keys().cloned().collect();
            Select::new(&i18n::t(keys::TEMPLATE_INSTALL_PROMPT_NAME), names)
                .prompt()
                .map_err(prompt_to_user_error)?
        }
    };

    let versions = catalog.get(&name).cloned().unwrap_or_default();
    if versions.is_empty() {
        return Err(ErrorEnvelope::user_error(
            format!("no installable versions for {name} in repo"),
            None,
            Some(&name),
            "",
        ));
    }

    let (version, picker_force) = match parsed.and_then(|t| t.version.clone()) {
        Some(v) => (Some(v), false),
        None => {
            require_interactive("template version")?;
            let labels = label_versions_with_status(snapshot, &name, &versions, dest_root);
            let prompt = i18n::tf(keys::TEMPLATE_INSTALL_PROMPT_VERSION, &[("name", &name)]);
            let chosen_label = Select::new(&prompt, labels.iter().map(|(l, _)| l.clone()).collect())
                .prompt()
                .map_err(prompt_to_user_error)?;
            let chosen_idx = labels
                .iter()
                .position(|(l, _)| l == &chosen_label)
                .ok_or_else(|| {
                    ErrorEnvelope::user_error(
                        "internal: version picker returned an unknown label",
                        None,
                        None,
                        "",
                    )
                })?;
            let (_, status) = &labels[chosen_idx];
            let v = versions[chosen_idx].clone();
            let force_flag = match status {
                VersionStatus::NotInstalled => false,
                VersionStatus::Installed
                | VersionStatus::Modified
                | VersionStatus::Outdated => {
                    let prompt = i18n::tf(
                        keys::TEMPLATE_INSTALL_CONFIRM_OVERWRITE,
                        &[("name", &name), ("version", &v)],
                    );
                    let proceed = Confirm::new(&prompt)
                        .with_default(false)
                        .prompt()
                        .map_err(prompt_to_user_error)?;
                    if !proceed {
                        return Err(ErrorEnvelope::user_error(
                            "install cancelled by user",
                            None,
                            None,
                            "",
                        ));
                    }
                    true
                }
            };
            (Some(v), force_flag)
        }
    };

    Ok((name, version, picker_force))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum VersionStatus {
    NotInstalled,
    Installed,
    Modified,
    Outdated,
}

/// Annotates each version with an installation status by comparing the
/// installed sidecar's `source_hash` against the on-disk re-hash (→ modified)
/// and against the snapshot's hash of the same version (→ outdated). Versions
/// without a sidecar are treated as not installed.
fn label_versions_with_status(
    snapshot: &RepoSnapshot,
    name: &str,
    versions: &[String],
    dest_root: &Path,
) -> Vec<(String, VersionStatus)> {
    versions
        .iter()
        .map(|v| {
            let status = classify_version(snapshot, name, v, dest_root);
            let label = match status {
                VersionStatus::NotInstalled => v.clone(),
                VersionStatus::Installed => {
                    format!("{v}  [{}]", i18n::t(keys::TEMPLATE_INSTALL_STATUS_INSTALLED))
                }
                VersionStatus::Modified => {
                    format!("{v}  [{}]", i18n::t(keys::TEMPLATE_INSTALL_STATUS_MODIFIED))
                }
                VersionStatus::Outdated => {
                    format!("{v}  [{}]", i18n::t(keys::TEMPLATE_INSTALL_STATUS_OUTDATED))
                }
            };
            (label, status)
        })
        .collect()
}

fn classify_version(
    snapshot: &RepoSnapshot,
    name: &str,
    version: &str,
    dest_root: &Path,
) -> VersionStatus {
    let installed_dir = dest_root.join(name).join(version);
    let Some(meta) = load_install_meta(&installed_dir) else {
        // Either not installed at all, or installed before the sidecar
        // existed. Either way, treat as "not installed" so the picker
        // doesn't flag it as modified for everyone upgrading the CLI.
        if installed_dir.join("template.toml").is_file() {
            return VersionStatus::Modified;
        }
        return VersionStatus::NotInstalled;
    };
    let local_hash = hash_dir(&installed_dir).unwrap_or_default();
    if local_hash != meta.source_hash {
        return VersionStatus::Modified;
    }
    let Some(repo_dir) = snapshot.template_dir(name, version) else {
        // Version was deleted upstream; "installed" is still accurate from
        // the user's perspective for this run.
        return VersionStatus::Installed;
    };
    let repo_hash = hash_dir(&repo_dir).unwrap_or_default();
    if repo_hash != meta.source_hash {
        VersionStatus::Outdated
    } else {
        VersionStatus::Installed
    }
}

/// Prompts the user to pick which subdirectories of the snapshot's shared
/// `doc/` to layer. Returns `[]` when:
/// * the template bundles its own `doc/` (we never overwrite author-shipped
///   doc with the shared bundle);
/// * the shared bundle has no per-category subdirectories;
/// * the session is non-interactive (agent / piped stdin);
/// * the user passed both name and version on the CLI (treated as a scripted
///   install — don't surprise the caller with a prompt).
fn pick_doc_categories(
    snapshot: &RepoSnapshot,
    name: &str,
    version: &str,
    parsed: Option<&TemplateRef>,
) -> Result<Vec<String>, ErrorEnvelope> {
    if snapshot.template_has_doc(name, version) {
        return Ok(Vec::new());
    }
    let cats = snapshot.shared_doc_categories();
    if cats.is_empty() {
        return Ok(Vec::new());
    }
    if parsed.and_then(|t| t.version.as_ref()).is_some() {
        return Ok(Vec::new());
    }
    if is_agent_active() || !std::io::stdin().is_terminal() {
        return Ok(Vec::new());
    }
    let picked = MultiSelect::new(&i18n::t(keys::TEMPLATE_INSTALL_PROMPT_DOC), cats)
        .with_default(&[])
        .prompt()
        .map_err(prompt_to_user_error)?;
    Ok(picked)
}

fn require_interactive(what: &'static str) -> Result<(), ErrorEnvelope> {
    if is_agent_active() || !std::io::stdin().is_terminal() {
        return Err(ErrorEnvelope::user_error(
            format!("cannot pick {what} without a TTY"),
            None,
            None,
            i18n::t(keys::ERROR_TEMPLATE_INSTALL_NEEDS_TTY),
        ));
    }
    Ok(())
}

fn prompt_to_user_error(e: inquire::InquireError) -> ErrorEnvelope {
    ErrorEnvelope::user_error(e.to_string(), None, None, "")
}

fn cmd_list() -> Result<(), ErrorEnvelope> {
    let installed = list_installed_templates();
    if installed.is_empty() {
        emit_success(Some(i18n::t(keys::TEMPLATE_LIST_EMPTY)));
        return Ok(());
    }
    let mut lines = String::new();
    for t in &installed {
        let desc = t.manifest.description.as_deref().unwrap_or("");
        let line = i18n::tf(
            keys::TEMPLATE_LIST_ENTRY,
            &[
                ("name", &t.name),
                ("version", &t.version),
                ("backend", t.manifest.backend.as_key()),
                ("frontend", t.manifest.frontend.as_key()),
                ("description", desc),
            ],
        );
        lines.push_str(&line);
        lines.push('\n');
    }
    emit_success(Some(lines.trim_end()));
    Ok(())
}

fn cmd_use(spec: &str, yes: bool) -> Result<(), ErrorEnvelope> {
    let target_ref = TemplateRef::parse(spec).map_err(|e| {
        ErrorEnvelope::user_error(format!("invalid template: {e}"), Some("template"), Some(spec), "")
    })?;
    let installed = find_template(&target_ref.name, target_ref.version.as_deref())?;

    let cwd = env::current_dir().map_err(|e| ErrorEnvelope {
        kind: Kind::ConfigError,
        msg: format!("cwd: {e}"),
        exit_code: Kind::ConfigError.exit_code(),
        hint: String::new(),
        details: serde_json::Map::new(),
    })?;
    let setup_path = project_setup_toml(&cwd);
    if !setup_path.exists() {
        return Err(ErrorEnvelope::user_error(
            format!("missing {}", setup_path.display()),
            None,
            None,
            i18n::t(keys::TEMPLATE_USE_MISSING_SETUP),
        ));
    }
    let mut cfg = load_setup_file(&setup_path)?;

    let needs_confirm = backend_or_frontend_differ(&cfg, &installed);
    if needs_confirm && !yes {
        let prompt = i18n::tf(
            keys::TEMPLATE_USE_CONFIRM,
            &[
                ("name", &installed.name),
                ("b", installed.manifest.backend.as_key()),
                ("f", installed.manifest.frontend.as_key()),
                ("cb", cfg.project.backend.as_key()),
                ("cf", cfg.project.frontend.as_key()),
            ],
        );
        if is_agent_active() || !std::io::stdin().is_terminal() {
            return Err(ErrorEnvelope::user_error(
                "template switch requires confirmation",
                None,
                None,
                prompt,
            ));
        }
        let proceed = Confirm::new(&prompt)
            .with_default(false)
            .prompt()
            .map_err(|e| ErrorEnvelope::user_error(e.to_string(), None, None, ""))?;
        if !proceed {
            return Ok(());
        }
    }

    cfg.project.backend = installed.manifest.backend.clone();
    cfg.project.frontend = installed.manifest.frontend.clone();
    cfg.project.template = Some(TemplateRef {
        name: installed.name.clone(),
        version: Some(installed.version.clone()),
    });
    write_setup(&setup_path, &cfg)?;

    let line = i18n::tf(
        keys::TEMPLATE_USE_APPLIED,
        &[
            ("name", &installed.name),
            ("version", &installed.version),
            ("path", &setup_path.display().to_string()),
        ],
    );
    emit_success(Some(&line));
    Ok(())
}

fn backend_or_frontend_differ(cfg: &SetupConfig, t: &InstalledTemplate) -> bool {
    cfg.project.backend != t.manifest.backend || cfg.project.frontend != t.manifest.frontend
}

fn write_setup(path: &Path, cfg: &SetupConfig) -> Result<(), ErrorEnvelope> {
    let body = cfg.to_toml_pretty()?;
    std::fs::write(path, body.as_bytes()).map_err(|e| ErrorEnvelope {
        kind: Kind::ConfigError,
        msg: format!("write {}: {e}", path.display()),
        exit_code: Kind::ConfigError.exit_code(),
        hint: String::new(),
        details: serde_json::Map::new(),
    })
}
