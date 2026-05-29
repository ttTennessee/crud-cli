//! `crud-cli template ...` — manage installed templates under
//! `~/.crud/templates/`.

use std::env;
use std::io::IsTerminal;
use std::path::Path;

use inquire::{Confirm, Select};

use crate::cli::agent_mode::is_agent_active;
use crate::cli::args::{exit_with_envelope, TemplateArgs, TemplateCommand};
use crate::cli::output::emit_success;
use crate::core::config::{load_setup_file, SetupConfig, TemplateRef};
use crate::core::error::{ErrorEnvelope, Kind};
use crate::core::global_config::GlobalConfig;
use crate::core::i18n::{self, keys};
use crate::core::paths::{global_config_toml, project_setup_toml};
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
    let (name, version) = resolve_name_and_version(&snapshot, parsed.as_ref())?;

    let installed = install_from_snapshot(&snapshot, &name, version.as_deref(), &dest_root, force)?;

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
/// `(Some(name), Some(version))` short-circuits both prompts.
fn resolve_name_and_version(
    snapshot: &RepoSnapshot,
    parsed: Option<&TemplateRef>,
) -> Result<(String, Option<String>), ErrorEnvelope> {
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

    let version = match parsed.and_then(|t| t.version.clone()) {
        Some(v) => Some(v),
        None => {
            // `template install <name>` always prompts; sole-version repos still
            // get a confirmation prompt, which costs ~one keypress and avoids a
            // surprising silent install.
            require_interactive("template version")?;
            let chosen = Select::new(
                &i18n::tf(keys::TEMPLATE_INSTALL_PROMPT_VERSION, &[("name", &name)]),
                versions,
            )
            .prompt()
            .map_err(prompt_to_user_error)?;
            Some(chosen)
        }
    };

    Ok((name, version))
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
