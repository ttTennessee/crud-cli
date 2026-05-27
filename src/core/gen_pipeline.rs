//! `gen` orchestration: template discovery, render, and atomic write.

use std::path::{Component, Path, PathBuf};

use crate::core::config::load_setup_file;
use crate::core::gen_run::GenRunParams;
use crate::core::error::ErrorEnvelope;
use crate::core::field_dsl;
use crate::core::fs_writer::{commit, plan, OverwriteContext, WriteTarget};
use crate::core::gen_context;
use crate::core::gen_input::GenInput;
use crate::core::gen_report::GenReport;
use crate::core::git_info;
use crate::core::template_engine;
use crate::core::template_loader::{self, TemplateEntry};

/**
 * Resolves the on-disk output path for a template (D-G28 layer 3).
 *
 * Rejects `..`, absolute segments, and strips a trailing `.hbs` from the filename.
 */
pub fn resolve_output_path(
    entry: &TemplateEntry,
    project_root: &Path,
) -> Result<PathBuf, ErrorEnvelope> {
    for component in entry.rel_path.components() {
        match component {
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                let mut details = serde_json::Map::new();
                details.insert(
                    "rel_path".into(),
                    serde_json::Value::String(entry.rel_path.to_string_lossy().into_owned()),
                );
                return Err(ErrorEnvelope::user_error_with_reason(
                    "path traversal in template output",
                    "path_traversal",
                    details,
                    "remove .. or absolute path from template location",
                ));
            }
            Component::Normal(_) | Component::CurDir => {}
        }
    }

    let parent = entry.rel_path.parent();
    let file_name = entry
        .rel_path
        .file_name()
        .map(|n| n.to_string_lossy())
        .unwrap_or_default();
    let stripped = file_name.strip_suffix(".hbs").unwrap_or(&file_name);
    let mut out_rel = parent.map(Path::to_path_buf).unwrap_or_default();
    out_rel.push(stripped);

    Ok(project_root.join(out_rel))
}

/// Runs the generation pipeline end-to-end.
pub fn run(params: GenRunParams) -> Result<GenReport, ErrorEnvelope> {
    let cwd = std::env::current_dir().map_err(|e| {
        ErrorEnvelope::user_error(
            format!("cwd: {e}"),
            None,
            None,
            "run inside a project directory",
        )
    })?;

    if params.file.is_some() {
        return Err(ErrorEnvelope::user_error_with_reason(
            "--file not yet supported",
            "file_input_not_yet_supported",
            serde_json::Map::new(),
            "--file lands in plan 02; use --fields for now",
        ));
    }

    let fields = field_dsl::parse_fields(&params.fields_src)?;
    let input = GenInput {
        name: params.name,
        table: params.table,
        package: params.package,
        fields,
    };

    let setup = load_setup_file(&cwd.join(".crud/setup.toml"))?;
    let git = git_info::read();
    let context = gen_context::build_context(&input, &setup, &git);

    let entries =
        template_loader::discover_templates(&cwd, params.type_filter.as_deref())?;

    let mut targets = Vec::new();
    for entry in &entries {
        let body = std::fs::read_to_string(&entry.abs_path).map_err(|e| {
            ErrorEnvelope::template_error(format!(
                "read {}: {e}",
                entry.abs_path.display()
            ))
        })?;
        let rendered = template_engine::render_template(&body, &context)?;
        let out = resolve_output_path(entry, &cwd)?;
        targets.push(WriteTarget {
            path: out,
            content: rendered.into_bytes(),
        });
    }

    if params.dry_run {
        return Ok(GenReport {
            written: vec![],
            skipped: targets.into_iter().map(|t| t.path).collect(),
            conflicts: vec![],
        });
    }

    let ctx = OverwriteContext {
        policy: setup.overwrite.overwrite_policy,
        force: params.force,
    };
    let write_plan = plan(&targets, ctx)?;
    commit(write_plan)?;

    Ok(GenReport {
        written: targets.into_iter().map(|t| t.path).collect(),
        skipped: vec![],
        conflicts: vec![],
    })
}
