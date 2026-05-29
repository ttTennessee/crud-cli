//! Template validation orchestrator (VAL-01..04, D-G20..D-G23).

use std::collections::BTreeSet;
use std::path::Path;

use handlebars::template::{HelperTemplate, Parameter, TemplateElement};
use handlebars::{Path as HbPath, PathSeg, Template, TemplateError};
use serde::Serialize;

use super::config::{EnabledTypes, RuntimeConfig, SetupConfig};
use super::paths::{project_setup_toml, project_setup_user_toml};
use super::error::ErrorEnvelope;
use super::i18n::{self, keys};
use super::field_dsl::Field;
use super::gen_context::{self, AsContextField, UserIdentity};
use super::git_info::GitInfo;
use super::template_engine;
use super::template_loader;
use super::template_meta;
use super::template_variables;

/// Issue category for structured validate output.
#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum IssueKind {
    SyntaxError,
    UnknownVariable,
    RenderError,
    MissingHelper,
    FrontMatterError,
    InvalidFilename,
    PathTraversal,
}

/// One validate finding (VAL-04 field names).
#[derive(Debug, Clone, Serialize)]
pub struct ValidateIssue {
    pub template_path: String,
    pub line: Option<u32>,
    pub column: Option<u32>,
    pub kind: IssueKind,
    pub variable: Option<String>,
    pub suggestion: Option<String>,
}

/// Success summary returned when no issues were found.
#[derive(Debug, Clone, Default, Serialize)]
pub struct ValidateReport {
    pub templates_checked: u32,
    pub templates_with_issues: u32,
    pub issue_count: u32,
}

/// Inputs for validate (keeps `core` free of `cli` types).
#[derive(Debug, Clone, Default)]
pub struct ValidateParams {
    pub type_filter: Option<Vec<String>>,
}

const BUILTINS: &[&str] = &[
    "model",
    "table",
    "package",
    "package_path",
    "fields",
    "model_snake",
    "model_pascal",
    "model_camel",
    "model_kebab",
    "git_user_name",
    "git_user_email",
    "user_name",
    "user_email",
    "date",
    "datetime",
    "year",
    "this",
    "@root",
];

const EACH_GLOBALS: &[&str] = &["@index", "@key", "@first", "@last"];

const FIELD_EACH_EXTRA: &[&str] = &[
    "name",
    "name_snake",
    "name_pascal",
    "name_camel",
    "name_kebab",
    "type",
    "is_pk",
    "nullable",
    "comment",
    "length",
    "unique",
    "default",
];

/**
 * Walks project templates and returns a report or aggregated `TemplateError`.
 *
 * Per-template fail-fast: syntax → variables → render; issues aggregate across templates.
 */
pub fn run(params: ValidateParams) -> Result<ValidateReport, ErrorEnvelope> {
    let cwd = std::env::current_dir().map_err(|e| {
        ErrorEnvelope::user_error(
            format!("cannot read current directory: {e}"),
            None,
            None,
            i18n::t(keys::ERROR_VALIDATE_CWD),
        )
    })?;
    let runtime = RuntimeConfig::load(
        &project_setup_toml(&cwd),
        &project_setup_user_toml(&cwd),
    )?;
    let setup = &runtime.project;

    let templates_root =
        crate::core::template_loader::resolve_templates_root(&cwd, setup)?;
    let schema = template_variables::load_schema(&templates_root)?;

    let implicit_filter = params
        .type_filter
        .clone()
        .or_else(|| implicit_type_prefixes(setup, runtime.enabled_types()));
    let entries =
        template_loader::discover_templates(&templates_root, implicit_filter.as_deref())?;
    let templates_checked = entries.len();

    let mut base_allow = build_base_allow_set(setup);
    for name in schema.names() {
        base_allow.insert(name.to_string());
    }
    let suggest_pool: Vec<String> = base_allow.iter().cloned().collect();

    let fixture_fields: [&dyn AsContextField; 3] = [
        &Field {
            name: "id".into(),
            ty: "Long".into(),
            is_pk: true,
            nullable: false,
        },
        &Field {
            name: "email".into(),
            ty: "String".into(),
            is_pk: false,
            nullable: false,
        },
        &Field {
            name: "created_at".into(),
            ty: "LocalDateTime".into(),
            is_pk: false,
            nullable: true,
        },
    ];
    let git = GitInfo::default();
    let user = UserIdentity {
        name: runtime.user.user.name.clone(),
        email: runtime.user.user.email.clone(),
    };
    let fixture_ctx = gen_context::build_context(
        "ValidateFixture",
        "validate_fixture",
        "com.example.validate",
        &fixture_fields,
        setup,
        &git,
        &user,
    )?;

    let mut issues = Vec::new();

    for entry in &entries {
        let rel = normalize_rel_path(&entry.rel_path);
        let body = std::fs::read_to_string(&entry.abs_path).map_err(|e| {
            ErrorEnvelope::template_error(format!("read {}: {e}", entry.abs_path.display()))
        })?;

        let meta = match template_meta::split_front_matter(&body) {
            Ok((m, _)) => Some(m),
            Err(err) => {
                issues.push(front_matter_issue(&rel, &err.msg));
                None
            }
        };

        let template = match Template::compile(&body) {
            Ok(t) => t,
            Err(err) => {
                issues.push(syntax_issue(&rel, &err));
                continue;
            }
        };

        if let Some(issue) = first_unknown_variable_issue(&template, &base_allow, &suggest_pool, &rel)
        {
            issues.push(issue);
            continue;
        }

        if let Some(issue) = render_issue(&body, &fixture_ctx, &rel) {
            issues.push(issue);
            continue;
        }

        if let Some(m) = meta {
            for issue in meta_path_issues(&m, &fixture_ctx, &rel) {
                issues.push(issue);
            }
            for issue in condition_issues(&m, &base_allow, &suggest_pool, &rel) {
                issues.push(issue);
            }
        }
    }

    if issues.is_empty() {
        return Ok(ValidateReport {
            templates_checked: templates_checked as u32,
            templates_with_issues: 0,
            issue_count: 0,
        });
    }

    let templates_with_issues = issues
        .iter()
        .map(|i| i.template_path.as_str())
        .collect::<BTreeSet<_>>()
        .len();
    let issue_count = issues.len();
    let summary = serde_json::json!({
        "templates_checked": templates_checked,
        "templates_with_issues": templates_with_issues,
        "issue_count": issue_count,
    });
    let issues_json = serde_json::to_value(&issues).map_err(|e| {
        ErrorEnvelope::template_error(format!("serialize validate issues: {e}"))
    })?;
    Err(ErrorEnvelope::template_error_with_issues(
        issues_json,
        summary,
    ))
}

fn normalize_rel_path(rel: &Path) -> String {
    rel.to_string_lossy().replace('\\', "/")
}

fn implicit_type_prefixes(project: &SetupConfig, enabled: EnabledTypes) -> Option<Vec<String>> {
    let aux_keys: Vec<String> = project.paths.aux.keys().cloned().collect();
    let mut prefixes: Vec<String> = match enabled {
        EnabledTypes::All => return None,
        EnabledTypes::Backend => {
            let mut v: Vec<String> = if project.project.backend.is_none() {
                Vec::new()
            } else {
                vec![project.project.backend.as_key().to_string()]
            };
            v.extend(aux_keys);
            v
        }
        EnabledTypes::Frontend => {
            if project.project.frontend.is_none() {
                Vec::new()
            } else {
                vec![project.project.frontend.as_key().to_string()]
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

fn build_base_allow_set(setup: &SetupConfig) -> BTreeSet<String> {
    let mut set = BTreeSet::new();
    for name in BUILTINS {
        set.insert((*name).to_string());
    }
    for name in EACH_GLOBALS {
        set.insert((*name).to_string());
    }
    for key in setup.variables.0.keys() {
        set.insert(key.clone());
    }
    set
}

fn effective_allow(base: &BTreeSet<String>, each_stack: &[BTreeSet<String>]) -> BTreeSet<String> {
    let mut allow = base.clone();
    for layer in each_stack {
        allow.extend(layer.iter().cloned());
    }
    allow
}

fn syntax_issue(rel: &str, err: &TemplateError) -> ValidateIssue {
    let (line, column) = err
        .pos()
        .map(|(l, c)| (Some(l as u32), Some(c as u32)))
        .unwrap_or((None, None));
    ValidateIssue {
        template_path: rel.to_string(),
        line,
        column,
        kind: IssueKind::SyntaxError,
        variable: None,
        suggestion: Some(err.reason().to_string()),
    }
}

fn first_unknown_variable_issue(
    template: &Template,
    base_allow: &BTreeSet<String>,
    suggest_pool: &[String],
    rel: &str,
) -> Option<ValidateIssue> {
    let mut each_stack: Vec<BTreeSet<String>> = Vec::new();
    walk_template(
        &template.elements,
        base_allow,
        &mut each_stack,
        suggest_pool,
        rel,
    )
}

fn walk_template(
    elements: &[TemplateElement],
    base_allow: &BTreeSet<String>,
    each_stack: &mut Vec<BTreeSet<String>>,
    suggest_pool: &[String],
    rel: &str,
) -> Option<ValidateIssue> {
    for element in elements {
        if let Some(issue) = walk_element(element, base_allow, each_stack, suggest_pool, rel) {
            return Some(issue);
        }
    }
    None
}

fn walk_element(
    element: &TemplateElement,
    base_allow: &BTreeSet<String>,
    each_stack: &mut Vec<BTreeSet<String>>,
    suggest_pool: &[String],
    rel: &str,
) -> Option<ValidateIssue> {
    match element {
        TemplateElement::Expression(ht) | TemplateElement::HtmlExpression(ht) => {
            check_helper_paths(ht, base_allow, each_stack, suggest_pool, rel)
        }
        TemplateElement::HelperBlock(ht) => {
            if let Some(issue) = check_helper_paths(ht, base_allow, each_stack, suggest_pool, rel) {
                return Some(issue);
            }
            let mut pushed = false;
            if is_each_on_fields(ht) {
                each_stack.push(field_each_allow());
                pushed = true;
            } else if is_each_block(ht) {
                each_stack.push(each_only_allow());
                pushed = true;
            }
            let mut issue = None;
            if let Some(tmpl) = &ht.template {
                issue = walk_template(&tmpl.elements, base_allow, each_stack, suggest_pool, rel);
            }
            if issue.is_none() {
                if let Some(tmpl) = &ht.inverse {
                    issue =
                        walk_template(&tmpl.elements, base_allow, each_stack, suggest_pool, rel);
                }
            }
            if pushed {
                each_stack.pop();
            }
            issue
        }
        _ => None,
    }
}

fn field_each_allow() -> BTreeSet<String> {
    let mut set = BTreeSet::new();
    for name in FIELD_EACH_EXTRA {
        set.insert((*name).to_string());
    }
    for name in EACH_GLOBALS {
        set.insert((*name).to_string());
    }
    set.insert("this".to_string());
    set
}

fn each_only_allow() -> BTreeSet<String> {
    let mut set = BTreeSet::new();
    for name in EACH_GLOBALS {
        set.insert((*name).to_string());
    }
    set.insert("this".to_string());
    set
}

fn is_each_on_fields(ht: &HelperTemplate) -> bool {
    helper_name(ht).as_deref() == Some("each")
        && ht
            .params
            .first()
            .and_then(first_segment_from_param)
            .as_deref()
            == Some("fields")
}

fn is_each_block(ht: &HelperTemplate) -> bool {
    helper_name(ht).as_deref() == Some("each")
}

fn is_simple_expression(ht: &HelperTemplate) -> bool {
    !ht.block && ht.params.is_empty() && ht.hash.is_empty()
}

fn helper_name(ht: &HelperTemplate) -> Option<String> {
    match &ht.name {
        Parameter::Name(n) => Some(n.clone()),
        Parameter::Path(p) => first_segment_from_path(p),
        _ => None,
    }
}

fn check_helper_paths(
    ht: &HelperTemplate,
    base_allow: &BTreeSet<String>,
    each_stack: &[BTreeSet<String>],
    suggest_pool: &[String],
    rel: &str,
) -> Option<ValidateIssue> {
    let allow = effective_allow(base_allow, each_stack);
    if is_simple_expression(ht) {
        if let Some(seg) = first_segment_from_param(&ht.name) {
            if let Some(issue) = check_segment(&seg, &allow, suggest_pool, rel) {
                return Some(issue);
            }
        }
    }
    for param in ht.params.iter().chain(ht.hash.values()) {
        if let Some(seg) = first_segment_from_param(param) {
            if let Some(issue) = check_segment(&seg, &allow, suggest_pool, rel) {
                return Some(issue);
            }
        }
    }
    None
}

fn check_segment(
    seg: &str,
    allow: &BTreeSet<String>,
    suggest_pool: &[String],
    rel: &str,
) -> Option<ValidateIssue> {
    if segment_allowed(seg, allow) {
        return None;
    }
    Some(ValidateIssue {
        template_path: rel.to_string(),
        line: None,
        column: None,
        kind: IssueKind::UnknownVariable,
        variable: Some(seg.to_string()),
        suggestion: did_you_mean(seg, suggest_pool),
    })
}

fn first_segment_from_param(param: &Parameter) -> Option<String> {
    match param {
        Parameter::Name(_) => None,
        Parameter::Path(p) => first_segment_from_path(p),
        Parameter::Subexpression(sub) => match sub.as_element() {
            TemplateElement::Expression(ht) => first_segment_from_param(&ht.name),
            _ => None,
        },
        Parameter::Literal(_) => None,
        _ => None,
    }
}

fn first_segment_from_path(path: &HbPath) -> Option<String> {
    match path {
        HbPath::Relative((segs, _)) => segs.first().and_then(|s| match s {
            PathSeg::Named(n) => Some(n.clone()),
            PathSeg::Ruled(_) => None,
            _ => None,
        }),
        HbPath::Local((_, name, _)) => Some(local_path_segment(name)),
    }
}

/// Normalizes each-loop locals (`@index` in source → `index` in AST).
fn local_path_segment(name: &str) -> String {
    if name.starts_with('@') {
        name.to_string()
    } else if matches!(name, "index" | "key" | "first" | "last" | "root") {
        format!("@{name}")
    } else {
        name.to_string()
    }
}

fn segment_allowed(seg: &str, allow: &BTreeSet<String>) -> bool {
    if allow.contains(seg) {
        return true;
    }
    if seg.starts_with('@') {
        let bare = seg.trim_start_matches('@');
        if allow.contains(bare) {
            return true;
        }
    } else if allow.contains(&format!("@{seg}")) {
        return true;
    }
    false
}

fn did_you_mean(seg: &str, pool: &[String]) -> Option<String> {
    let mut best: Option<(&str, usize)> = None;
    for candidate in pool {
        let dist = strsim::levenshtein(seg, candidate);
        if dist <= 2 {
            match best {
                Some((_, d)) if dist >= d => {}
                _ => best = Some((candidate.as_str(), dist)),
            }
        }
    }
    best.map(|(name, _)| format!("did you mean '{name}'?"))
}

fn front_matter_issue(rel: &str, msg: &str) -> ValidateIssue {
    ValidateIssue {
        template_path: rel.to_string(),
        line: None,
        column: None,
        kind: IssueKind::FrontMatterError,
        variable: None,
        suggestion: Some(msg.to_string()),
    }
}

/// Validates `generateWhen`/`skipWhen` conditions: a typo'd or undeclared
/// variable would silently evaluate falsy at gen time (engine is non-strict),
/// skipping the file with no error — so we catch unknown vars here.
fn condition_issues(
    meta: &template_meta::TemplateMeta,
    base_allow: &BTreeSet<String>,
    suggest_pool: &[String],
    rel: &str,
) -> Vec<ValidateIssue> {
    let mut out = Vec::new();
    for expr in [meta.generate_when.as_deref(), meta.skip_when.as_deref()]
        .into_iter()
        .flatten()
    {
        let wrapped = format!("{{{{#if {expr}}}}}1{{{{/if}}}}");
        match Template::compile(&wrapped) {
            Ok(template) => {
                if let Some(issue) =
                    first_unknown_variable_issue(&template, base_allow, suggest_pool, rel)
                {
                    out.push(issue);
                }
            }
            Err(err) => out.push(front_matter_issue(rel, err.reason().to_string().as_str())),
        }
    }
    out
}

fn meta_path_issues(
    meta: &template_meta::TemplateMeta,
    ctx: &serde_json::Value,
    rel: &str,
) -> Vec<ValidateIssue> {
    let mut out = Vec::new();
    if let Some(tpl) = meta.filename.as_deref() {
        if let Ok(rendered) = template_engine::render_template(tpl, ctx) {
            if rendered.contains('/') || rendered.contains('\\') {
                out.push(ValidateIssue {
                    template_path: rel.to_string(),
                    line: None,
                    column: None,
                    kind: IssueKind::InvalidFilename,
                    variable: Some(rendered),
                    suggestion: Some(
                        "filename must be a single path segment; use basePath for directories"
                            .into(),
                    ),
                });
            }
        }
    }
    if let Some(tpl) = meta.base_path.as_deref() {
        if let Ok(rendered) = template_engine::render_template(tpl, ctx) {
            if base_path_has_traversal(&rendered) {
                out.push(ValidateIssue {
                    template_path: rel.to_string(),
                    line: None,
                    column: None,
                    kind: IssueKind::PathTraversal,
                    variable: Some(rendered),
                    suggestion: Some(
                        "basePath must be a relative path with no '..' segments".into(),
                    ),
                });
            }
        }
    }
    out
}

fn base_path_has_traversal(rendered: &str) -> bool {
    let normalized = rendered.replace('\\', "/");
    if std::path::Path::new(&normalized).is_absolute() {
        return true;
    }
    normalized.split('/').any(|seg| seg == "..")
}

fn render_issue(
    body: &str,
    ctx: &serde_json::Value,
    rel: &str,
) -> Option<ValidateIssue> {
    match template_engine::render_template(body, ctx) {
        Ok(rendered) => {
            if let Some(idx) = rendered.find("{{") {
                let tail = &rendered[idx..];
                return Some(ValidateIssue {
                    template_path: rel.to_string(),
                    line: None,
                    column: None,
                    kind: IssueKind::RenderError,
                    variable: extract_residue_handle(tail),
                    suggestion: Some("unrendered handlebars residue in output".into()),
                });
            }
            None
        }
        Err(envelope) => {
            let s = envelope.msg;
            let lower = s.to_lowercase();
            let kind = if lower.contains("helper not found")
                || lower.contains("helper not defined")
                || lower.contains("unknown helper")
                || lower.contains("not registered")
            {
                IssueKind::MissingHelper
            } else {
                IssueKind::RenderError
            };
            Some(ValidateIssue {
                template_path: rel.to_string(),
                line: None,
                column: None,
                kind,
                variable: extract_helper_name(&s),
                suggestion: Some(s),
            })
        }
    }
}

fn extract_residue_handle(fragment: &str) -> Option<String> {
    let rest = fragment.strip_prefix("{{")?;
    let trimmed = rest.trim_start_matches('#').trim_start_matches('/').trim();
    let end = trimmed
        .find(|c: char| c.is_whitespace() || c == '}' || c == '(')
        .unwrap_or(trimmed.len());
    let name = trimmed.get(..end)?.trim();
    if name.is_empty() {
        None
    } else {
        Some(name.to_string())
    }
}

fn extract_helper_name(msg: &str) -> Option<String> {
    if let Some(start) = msg.find('\'') {
        let rest = &msg[start + 1..];
        if let Some(end) = rest.find('\'') {
            let name = rest.get(..end)?.trim();
            if !name.is_empty() {
                return Some(name.to_string());
            }
        }
    }
    const MARKERS: &[&str] = &[
        "Helper not found ",
        "helper not found ",
        "Decorator not found ",
    ];
    for marker in MARKERS {
        if let Some(idx) = msg.find(marker) {
            let tail = msg[idx + marker.len()..].trim();
            let name: String = tail
                .chars()
                .take_while(|c| c.is_ascii_alphanumeric() || *c == '_' || *c == '-')
                .collect();
            if !name.is_empty() {
                return Some(name);
            }
        }
    }
    None
}
