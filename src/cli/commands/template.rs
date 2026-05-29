//! `crud-cli template ...` — manage installed templates under
//! `~/.crud/templates/`.

use std::env;
use std::io::IsTerminal;
use std::path::Path;

use inquire::Confirm;

use crate::cli::agent_mode::is_agent_active;
use crate::cli::args::{exit_with_envelope, TemplateArgs, TemplateCommand};
use crate::cli::output::emit_success;
use crate::core::config::{load_setup_file, SetupConfig, TemplateRef};
use crate::core::error::{ErrorEnvelope, Kind};
use crate::core::global_config::GlobalConfig;
use crate::core::i18n::{self, keys};
use crate::core::paths::{global_config_toml, project_setup_toml};
use crate::core::template_installer::{install_template, RepoSpec};
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
            match cmd_install(&name, repo.as_deref(), force) {
                Ok(()) => 0,
                Err(env) => exit_with_envelope(&env),
            }
        }
    }
}

fn cmd_install(spec: &str, repo_override: Option<&str>, force: bool) -> Result<(), ErrorEnvelope> {
    let target_ref = TemplateRef::parse(spec).map_err(|e| {
        ErrorEnvelope::user_error(format!("invalid template: {e}"), Some("template"), Some(spec), "")
    })?;

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

    let dest_root = global_templates_root().ok_or_else(|| ErrorEnvelope {
        kind: Kind::ConfigError,
        msg: "cannot resolve ~/.crud/templates".into(),
        exit_code: Kind::ConfigError.exit_code(),
        hint: String::new(),
        details: serde_json::Map::new(),
    })?;

    let starting = i18n::tf(
        keys::TEMPLATE_INSTALL_STARTING,
        &[("name", &target_ref.name), ("repo", &repo.display())],
    );
    emit_success(Some(&starting));

    let installed = install_template(
        &target_ref.name,
        target_ref.version.as_deref(),
        &repo,
        &dest_root,
        force,
    )?;

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
