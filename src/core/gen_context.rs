//! Handlebars JSON context builder (D-G04, D-G06, D-G27).

use convert_case::{Case, Casing};
use serde_json::{Map, Value};
use std::sync::OnceLock;
use time::format_description::FormatItem;
use time::macros::format_description;
use time::OffsetDateTime;

use super::config::SetupConfig;
use super::error::ErrorEnvelope;
use super::field_dsl::{Field, RESERVED_VARIABLE_NAMES};
use super::gen_input::GenInput;
use super::git_info::GitInfo;

static EMPTY_EXTRA: OnceLock<Map<String, Value>> = OnceLock::new();

fn empty_extra() -> &'static Map<String, Value> {
    EMPTY_EXTRA.get_or_init(Map::new)
}

const DATE_FMT: &[FormatItem<'static>] = format_description!("[year]-[month]-[day]");
const DATETIME_FMT: &[FormatItem<'static>] =
    format_description!("[year]-[month]-[day] [hour]:[minute]:[second]");
const YEAR_FMT: &[FormatItem<'static>] = format_description!("[year]");

/// User identity surfaced to templates (`{{user_name}}`/`{{user_email}}`).
///
/// Sourced from `.crud/setup.user.toml`; falls back to git config so templates
/// keep working before a user runs the wizard.
#[derive(Debug, Clone, Default)]
pub struct UserIdentity {
    pub name: String,
    pub email: String,
}

/// Common field surface for DSL [`Field`] and JSON [`FieldSpec`] (Plan 02).
pub trait AsContextField {
    fn name(&self) -> &str;
    fn ty(&self) -> &str;
    fn is_pk(&self) -> bool;
    fn nullable(&self) -> bool;
    fn extra(&self) -> &Map<String, Value>;
}

impl AsContextField for Field {
    fn name(&self) -> &str {
        &self.name
    }
    fn ty(&self) -> &str {
        &self.ty
    }
    fn is_pk(&self) -> bool {
        self.is_pk
    }
    fn nullable(&self) -> bool {
        self.nullable
    }
    fn extra(&self) -> &Map<String, Value> {
        empty_extra()
    }
}

impl AsContextField for super::gen_input::FieldSpec {
    fn name(&self) -> &str {
        &self.name
    }
    fn ty(&self) -> &str {
        &self.ty
    }
    fn is_pk(&self) -> bool {
        self.is_pk
    }
    fn nullable(&self) -> bool {
        self.nullable
    }
    fn extra(&self) -> &Map<String, Value> {
        &self.extra
    }
}

/**
 * Builds the top-level render context for templates.
 *
 * Merges `[variables]` from setup at the top level (D-G27).
 */
pub fn build_context(
    name: &str,
    table: &str,
    package: &str,
    fields: &[&dyn AsContextField],
    setup: &SetupConfig,
    git: &GitInfo,
    user: &UserIdentity,
) -> Result<Value, ErrorEnvelope> {
    let model = name;
    let mut map = Map::new();
    map.insert("model".into(), Value::String(model.to_string()));
    map.insert(
        "model_pascal".into(),
        Value::String(model.to_case(Case::Pascal)),
    );
    map.insert(
        "model_snake".into(),
        Value::String(model.to_case(Case::Snake)),
    );
    map.insert(
        "model_camel".into(),
        Value::String(model.to_case(Case::Camel)),
    );
    map.insert(
        "model_kebab".into(),
        Value::String(model.to_case(Case::Kebab)),
    );
    map.insert("table".into(), Value::String(table.to_string()));
    map.insert("package".into(), Value::String(package.to_string()));
    map.insert(
        "package_path".into(),
        Value::String(package.replace('.', "/")),
    );
    map.insert(
        "fields".into(),
        Value::Array(fields.iter().map(|f| field_to_json(*f)).collect()),
    );
    map.insert("git_user_name".into(), Value::String(git.user_name.clone()));
    map.insert(
        "git_user_email".into(),
        Value::String(git.user_email.clone()),
    );

    let user_name = if user.name.trim().is_empty() {
        git.user_name.clone()
    } else {
        user.name.clone()
    };
    let user_email = if user.email.trim().is_empty() {
        git.user_email.clone()
    } else {
        user.email.clone()
    };
    map.insert("user_name".into(), Value::String(user_name));
    map.insert("user_email".into(), Value::String(user_email));

    let now = OffsetDateTime::now_local().unwrap_or_else(|_| OffsetDateTime::now_utc());
    map.insert(
        "date".into(),
        Value::String(now.format(DATE_FMT).unwrap_or_default()),
    );
    map.insert(
        "datetime".into(),
        Value::String(now.format(DATETIME_FMT).unwrap_or_default()),
    );
    map.insert(
        "year".into(),
        Value::String(now.format(YEAR_FMT).unwrap_or_default()),
    );

    for (key, val) in &setup.variables.0 {
        if RESERVED_VARIABLE_NAMES.contains(&key.as_str()) {
            let mut details = serde_json::Map::new();
            details.insert("variable".into(), Value::String(key.clone()));
            details.insert(
                "reason".into(),
                Value::String("variable_shadows_builtin".into()),
            );
            return Err(ErrorEnvelope::template_error_with_reason(
                format!("variable shadows built-in: {key}"),
                details,
                "remove or rename the variable in setup.toml [variables]",
            ));
        }
        let json = serde_json::to_value(val).map_err(|e| {
            ErrorEnvelope::template_error(format!("variable {key} to JSON: {e}"))
        })?;
        map.insert(key.clone(), json);
    }

    Ok(Value::Object(map))
}

/// Thin wrapper for DSL-only [`GenInput`] call sites.
pub fn build_context_from_input(
    input: &GenInput,
    setup: &SetupConfig,
    git: &GitInfo,
    user: &UserIdentity,
) -> Result<Value, ErrorEnvelope> {
    let refs: Vec<&dyn AsContextField> = input
        .fields
        .iter()
        .map(|f| f as &dyn AsContextField)
        .collect();
    build_context(
        &input.name,
        &input.table,
        &input.package,
        &refs,
        setup,
        git,
        user,
    )
}

fn field_to_json(field: &dyn AsContextField) -> Value {
    let name = field.name();
    let mut m = Map::new();
    m.insert("name".into(), Value::String(name.to_string()));
    m.insert(
        "name_pascal".into(),
        Value::String(name.to_case(Case::Pascal)),
    );
    m.insert(
        "name_snake".into(),
        Value::String(name.to_case(Case::Snake)),
    );
    m.insert(
        "name_camel".into(),
        Value::String(name.to_case(Case::Camel)),
    );
    m.insert(
        "name_kebab".into(),
        Value::String(name.to_case(Case::Kebab)),
    );
    m.insert("type".into(), Value::String(field.ty().to_string()));
    m.insert("is_pk".into(), Value::Bool(field.is_pk()));
    m.insert("nullable".into(), Value::Bool(field.nullable()));
    for (k, v) in field.extra() {
        m.insert(k.clone(), v.clone());
    }
    Value::Object(m)
}
