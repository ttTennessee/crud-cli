//! `gen` orchestration: template discovery, render, and atomic write.

use std::path::{Component, Path, PathBuf};

use serde_json::Value;

use crate::core::config::{
    EnabledTypes, OutputsSection, OverwritePolicy, RuntimeConfig, SetupConfig,
};
use crate::core::paths::{project_setup_toml, project_setup_user_toml};
use crate::core::gen_run::GenRunParams;
use crate::core::error::ErrorEnvelope;
use crate::core::i18n::{self, keys};
use crate::core::field_dsl;
use crate::core::fs_writer::{commit, plan, OverwriteContext, WriteTarget};
use crate::core::gen_context::{self, AsContextField, UserIdentity};
use crate::core::gen_input::{GenCliOverrides, GenInput};
use crate::core::gen_report::{DryRunLine, GenReport};
use crate::core::git_info;
use crate::core::template_engine::{self, TypeMapBinding};
use crate::core::template_loader::{self, TemplateEntry};
use crate::core::template_meta::{self, TemplateMeta};
use crate::core::template_variables;
use crate::core::type_map;
use std::collections::BTreeMap;
use std::sync::Arc;

struct ResolvedTarget {
    path: PathBuf,
    content: Vec<u8>,
    meta: TemplateMeta,
}

/**
 * Resolves the on-disk output path (D-G28 layers 1–3).
 *
 * Enforces path traversal rejection after Handlebars render.
 */
pub fn resolve_output_path(
    entry: &TemplateEntry,
    meta: &TemplateMeta,
    outputs: &OutputsSection,
    context: &Value,
    project_root: &Path,
    output_override: Option<&Path>,
    setup: &SetupConfig,
) -> Result<PathBuf, ErrorEnvelope> {
    if meta
        .filename
        .as_deref()
        .is_some_and(|f| f.contains('/') || f.contains('\\'))
    {
        return Err(ErrorEnvelope::user_error_with_reason(
            "filename contains path separator",
            "filename_has_slash",
            serde_json::Map::new(),
            i18n::t(keys::ERROR_GEN_FILENAME_SLASH),
        ));
    }

    let rel_key = normalize_rel_path(&entry.rel_path);
    for component in entry.rel_path.components() {
        match component {
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                return Err(path_traversal_error(Some(&rel_key)));
            }
            Component::Normal(_) | Component::CurDir => {}
        }
    }

    let rendered_rel: String = if meta.base_path.is_some() || meta.filename.is_some() {
        let raw_base = match &meta.base_path {
            Some(bp) => template_engine::render_template(bp, context)?,
            None => entry
                .rel_path
                .parent()
                .map(normalize_rel_path)
                .filter(|s| !s.is_empty())
                .unwrap_or_default(),
        };
        let base = rebase_framework_prefix(&raw_base, setup).unwrap_or(raw_base);
        let file = match &meta.filename {
            Some(fn_tpl) => template_engine::render_template(fn_tpl, context)?,
            None => {
                let file_name = entry
                    .rel_path
                    .file_name()
                    .map(|n| n.to_string_lossy())
                    .unwrap_or_default();
                file_name
                    .strip_suffix(".hbs")
                    .unwrap_or(&file_name)
                    .to_string()
            }
        };
        if base.is_empty() {
            file
        } else {
            format!("{base}/{file}")
        }
    } else if let Some(template_str) = outputs.0.get(&rel_key) {
        template_engine::render_template(template_str, context)?
    } else {
        layer3_rendered_rel(entry, output_override, setup)
    };

    let rel_path = PathBuf::from(&rendered_rel);
    assert_safe_output_path(&rel_path, project_root, Some(&rel_key))
}

fn layer3_rendered_rel(
    entry: &TemplateEntry,
    output_override: Option<&Path>,
    setup: &SetupConfig,
) -> String {
    let mirror = source_mirror_rel(entry);
    if let Some(root) = output_override {
        let root_rel = normalize_rel_path(root);
        if mirror.is_empty() {
            root_rel
        } else {
            format!("{root_rel}/{mirror}")
        }
    } else if let Some(fw) = framework_layer3_rel(setup, &mirror) {
        fw
    } else {
        mirror
    }
}

/// Returns the directory `gen` should walk for `.hbs` templates. When the
/// project pins a global template, resolve to `~/.crud/templates/<n>/<v>/`;
/// otherwise fall back to project-local `.crud/templates/`.
fn resolve_templates_root(cwd: &Path, setup: &SetupConfig) -> Result<PathBuf, ErrorEnvelope> {
    if let Some(tref) = &setup.project.template {
        let installed = crate::core::template_meta_global::find_template(
            &tref.name,
            tref.version.as_deref(),
        )?;
        return Ok(installed.path);
    }
    Ok(cwd.join(".crud/templates"))
}

fn rebase_framework_prefix(rel: &str, setup: &SetupConfig) -> Option<String> {
    let (head, tail) = match rel.split_once('/') {
        Some((h, t)) => (h, Some(t)),
        None => (rel, None),
    };
    let base = setup.paths.lookup(head)?;
    Some(match tail {
        None => base.to_string(),
        Some(rest) if rest.is_empty() => base.to_string(),
        Some(rest) => format!("{base}/{rest}"),
    })
}

fn framework_layer3_rel(setup: &SetupConfig, mirror_rel: &str) -> Option<String> {
    let (head, _) = mirror_rel.split_once('/')?;
    let base = setup.paths.lookup(head)?;
    Some(join_base_strip_prefix(
        base,
        mirror_rel,
        &format!("{head}/"),
    ))
}

fn join_base_strip_prefix(base: &str, rel: &str, prefix: &str) -> String {
    let rest = rel.strip_prefix(prefix).unwrap_or(rel).trim_start_matches('/');
    if rest.is_empty() {
        base.to_string()
    } else {
        format!("{base}/{rest}")
    }
}

fn source_mirror_rel(entry: &TemplateEntry) -> String {
    for component in entry.rel_path.components() {
        if matches!(
            component,
            Component::ParentDir | Component::RootDir | Component::Prefix(_)
        ) {
            return String::new();
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
    normalize_rel_path(&out_rel)
}

fn normalize_rel_path(rel: &Path) -> String {
    rel.to_string_lossy().replace('\\', "/")
}

fn assert_safe_output_path(
    rel: &Path,
    project_root: &Path,
    rel_hint: Option<&str>,
) -> Result<PathBuf, ErrorEnvelope> {
    if rel.is_absolute() {
        return Err(path_traversal_error(rel_hint));
    }
    for component in rel.components() {
        match component {
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                return Err(path_traversal_error(rel_hint));
            }
            Component::Normal(_) | Component::CurDir => {}
        }
    }
    Ok(project_root.join(rel))
}

fn path_traversal_error(rel_hint: Option<&str>) -> ErrorEnvelope {
    let mut details = serde_json::Map::new();
    if let Some(r) = rel_hint {
        details.insert("rel_path".into(), serde_json::Value::String(r.to_string()));
    }
    ErrorEnvelope::user_error_with_reason(
        "path traversal in template output",
        "path_traversal",
        details,
        i18n::t(keys::ERROR_GEN_PATH_TRAVERSAL),
    )
}

fn effective_policy(meta: &TemplateMeta, overwrite: OverwritePolicy) -> OverwritePolicy {
    meta.overwrite.unwrap_or(overwrite)
}

/// Implicit `--type` prefixes derived from user.enabled-types and project stacks.
fn implicit_type_prefixes(project: &SetupConfig, enabled: EnabledTypes) -> Option<Vec<String>> {
    let backend_key = project.project.backend.as_key();
    let frontend_key = project.project.frontend.as_key();
    let aux_keys: Vec<String> = project.paths.aux.keys().cloned().collect();
    let mut prefixes: Vec<String> = match enabled {
        EnabledTypes::All => return None,
        EnabledTypes::Backend => {
            let mut v: Vec<String> = if project.project.backend.is_none() {
                Vec::new()
            } else {
                vec![backend_key.to_string()]
            };
            v.extend(aux_keys);
            v
        }
        EnabledTypes::Frontend => {
            if project.project.frontend.is_none() {
                Vec::new()
            } else {
                vec![frontend_key.to_string()]
            }
        }
    };
    prefixes.sort();
    prefixes.dedup();
    if prefixes.is_empty() {
        None
    } else {
        Some(prefixes)
    }
}

fn allows_overwrite(policy: OverwritePolicy, force: bool, path: &Path) -> bool {
    if !path.exists() {
        return true;
    }
    match policy {
        OverwritePolicy::Never => false,
        OverwritePolicy::ForceOnly => force,
        OverwritePolicy::Always => true,
    }
}

/// Runs the generation pipeline end-to-end.
pub fn run(params: GenRunParams) -> Result<GenReport, ErrorEnvelope> {
    let cwd = std::env::current_dir().map_err(|e| {
        ErrorEnvelope::user_error(
            format!("cwd: {e}"),
            None,
            None,
            i18n::t(keys::ERROR_GEN_CWD),
        )
    })?;

    let runtime = RuntimeConfig::load(
        &project_setup_toml(&cwd),
        &project_setup_user_toml(&cwd),
    )?;
    let setup = &runtime.project;
    let overwrite_policy = runtime.overwrite_policy();
    if let Some(ref out) = params.output_dir {
        assert_safe_output_path(out, &cwd, Some("output"))?;
    }
    let git = git_info::read();
    let user = UserIdentity {
        name: runtime.user.user.name.clone(),
        email: runtime.user.user.email.clone(),
    };

    let schema = template_variables::load_schema(&cwd)?;

    let (mut context, json_vars) = if let Some(ref path) = params.file {
        let loaded = super::gen_input::load_gen_input_with_specs_from_json(
            path,
            GenCliOverrides {
                name: params.name.clone(),
                package: params.package.clone(),
                table: params.table.clone(),
            },
        )?;
        let refs: Vec<&dyn AsContextField> = loaded
            .field_specs
            .iter()
            .map(|s| s as &dyn AsContextField)
            .collect();
        let ctx = gen_context::build_context(
            &loaded.input.name,
            &loaded.input.table,
            &loaded.input.package,
            &refs,
            setup,
            &git,
            &user,
        )?;
        (ctx, loaded.variables)
    } else {
        let fields = field_dsl::parse_fields(
            params
                .fields_src
                .as_deref()
                .ok_or_else(|| missing_pipeline_input("fields"))?,
        )?;
        let input = GenInput {
            name: params
                .name
                .clone()
                .ok_or_else(|| missing_pipeline_input("name"))?,
            table: params
                .table
                .clone()
                .ok_or_else(|| missing_pipeline_input("table"))?,
            package: params
                .package
                .clone()
                .ok_or_else(|| missing_pipeline_input("package"))?,
            fields,
        };
        let ctx = gen_context::build_context_from_input(&input, setup, &git, &user)?;
        (ctx, std::collections::BTreeMap::new())
    };

    let resolved_vars = template_variables::merge_values(&schema, &params.cli_vars, &json_vars)?;
    if let Some(obj) = context.as_object_mut() {
        for (k, v) in resolved_vars {
            obj.insert(k, v);
        }
    }

    let implicit_filter = params
        .type_filter
        .clone()
        .or_else(|| implicit_type_prefixes(setup, runtime.enabled_types()));
    let templates_root = resolve_templates_root(&cwd, setup)?;
    let entries =
        template_loader::discover_templates(&templates_root, implicit_filter.as_deref())?;
    let fallback = setup.type_map.fallback.clone();
    let mut bundle_cache: BTreeMap<String, Option<Arc<BTreeMap<String, String>>>> = BTreeMap::new();

    let mut resolved = Vec::new();
    for entry in &entries {
        let raw = std::fs::read_to_string(&entry.abs_path).map_err(|e| {
            ErrorEnvelope::template_error(format!(
                "read {}: {e}",
                entry.abs_path.display()
            ))
        })?;
        let (meta, body) = template_meta::split_front_matter(&raw)?;
        let bundle = entry
            .rel_path
            .components()
            .next()
            .and_then(|c| match c {
                Component::Normal(s) => s.to_str().map(|s| s.to_string()),
                _ => None,
            });
        let map = if let Some(name) = bundle.as_deref() {
            if let Some(slot) = bundle_cache.get(name) {
                slot.clone()
            } else {
                let loaded = type_map::load_for_bundle(&templates_root, name)?
                    .map(Arc::new);
                bundle_cache.insert(name.to_string(), loaded.clone());
                loaded
            }
        } else {
            None
        };
        let binding = TypeMapBinding {
            bundle: bundle.clone(),
            map,
            fallback: fallback.clone(),
        };
        let rendered = template_engine::render_template_with_type_map(&body, &context, binding)?;
        let out = resolve_output_path(
            entry,
            &meta,
            &setup.templates.outputs,
            &context,
            &cwd,
            params.output_dir.as_deref(),
            &setup,
        )?;
        resolved.push(ResolvedTarget {
            path: out,
            content: rendered.into_bytes(),
            meta,
        });
    }

    if params.dry_run {
        let mut conflicts = Vec::new();
        let mut skipped = Vec::new();
        let mut dry_run_lines = Vec::new();
        for t in &resolved {
            skipped.push(t.path.clone());
            let policy = effective_policy(&t.meta, overwrite_policy);
            let conflict = !allows_overwrite(policy, params.force, &t.path);
            if conflict {
                conflicts.push(t.path.clone());
            }
            let line_count = t.content.iter().filter(|&&b| b == b'\n').count() + 1;
            dry_run_lines.push(DryRunLine {
                path: t.path.clone(),
                line_count,
                conflict,
            });
        }
        return Ok(GenReport {
            written: vec![],
            skipped,
            conflicts,
            dry_run_lines,
        });
    }

    let targets: Vec<WriteTarget> = resolved
        .iter()
        .map(|t| WriteTarget {
            path: t.path.clone(),
            content: t.content.clone(),
        })
        .collect();

    for t in &resolved {
        let policy = effective_policy(&t.meta, overwrite_policy);
        if !allows_overwrite(policy, params.force, &t.path) {
            return Err(ErrorEnvelope::file_conflict(
                format!("file exists: {}", t.path.display()),
                &t.path,
            ));
        }
    }

    let write_plan = plan(
        &targets,
        OverwriteContext {
            policy: OverwritePolicy::Always,
            force: true,
        },
    )?;
    commit(write_plan)?;

    Ok(GenReport {
        written: resolved.into_iter().map(|t| t.path).collect(),
        skipped: vec![],
        conflicts: vec![],
        dry_run_lines: vec![],
    })
}

fn missing_pipeline_input(flag: &'static str) -> ErrorEnvelope {
    let mut details = serde_json::Map::new();
    details.insert("flag".into(), serde_json::Value::String(flag.to_string()));
    ErrorEnvelope::user_error_with_reason(
        format!("missing {flag} for generation"),
        "missing_field",
        details,
        i18n::tf(keys::ERROR_GEN_MISSING_INPUT, &[("flag", flag)]),
    )
}
