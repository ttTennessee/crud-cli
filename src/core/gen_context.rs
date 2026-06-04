//! Handlebars JSON context builder (D-G04, D-G06, D-G27).

use convert_case::{Case, Casing};
use serde_json::{Map, Value};
use std::sync::OnceLock;
use time::format_description::FormatItem;
use time::macros::format_description;
use time::OffsetDateTime;

use super::config::SetupConfig;
use super::error::ErrorEnvelope;
use super::i18n::{self, keys};
use super::field_dsl::{Field, RESERVED_VARIABLE_NAMES};
use super::gen_input::GenInput;
use super::git_info::GitInfo;

static EMPTY_EXTRA: OnceLock<Map<String, Value>> = OnceLock::new();

/// Optional sub-table slice passed into [`build_context`].
pub struct SubTableContext<'a> {
    pub name: &'a str,
    pub table: &'a str,
    pub table_comment: &'a str,
    pub fk_field: &'a str,
    pub fields: &'a [&'a dyn AsContextField],
}

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

/// Common field surface for DSL [`Field`] and JSON [`FieldSpec`].
///
/// `comment`/`length`/`unique`/`default` originate in the JSON [`FieldSpec`];
/// the DSL [`Field`] has no syntax for them and returns neutral defaults.
pub trait AsContextField {
    fn name(&self) -> &str;
    fn ty(&self) -> &str;
    fn is_pk(&self) -> bool;
    fn required(&self) -> bool;
    fn comment(&self) -> &str;
    fn length(&self) -> Option<u32>;
    fn unique(&self) -> bool;
    fn default_value(&self) -> Option<&Value>;
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
    fn required(&self) -> bool {
        false
    }
    fn comment(&self) -> &str {
        ""
    }
    fn length(&self) -> Option<u32> {
        None
    }
    fn unique(&self) -> bool {
        false
    }
    fn default_value(&self) -> Option<&Value> {
        None
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
    fn required(&self) -> bool {
        if self.required {
            return true;
        }
        self.extra
            .get("required")
            .and_then(Value::as_bool)
            .unwrap_or(false)
    }
    fn comment(&self) -> &str {
        &self.comment
    }
    fn length(&self) -> Option<u32> {
        self.length
    }
    fn unique(&self) -> bool {
        self.unique
    }
    fn default_value(&self) -> Option<&Value> {
        self.default.as_ref()
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
#[allow(clippy::too_many_arguments)]
pub fn build_context(
    name: &str,
    table: &str,
    package: &str,
    table_comment: &str,
    fields: &[&dyn AsContextField],
    sub: Option<&SubTableContext<'_>>,
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
    map.insert(
        "table_comment".into(),
        Value::String(table_comment.to_string()),
    );
    map.insert("package".into(), Value::String(package.to_string()));
    map.insert(
        "package_path".into(),
        Value::String(package.replace('.', "/")),
    );
    map.insert(
        "fields".into(),
        Value::Array(fields.iter().map(|f| field_to_json(*f)).collect()),
    );
    insert_pk_builtins(&mut map, fields);
    insert_sub_builtins(&mut map, sub);
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
                i18n::t(keys::ERROR_VARIABLE_SHADOWS_BUILTIN),
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
    if let Some(sub) = &input.sub {
        let sub_field_refs: Vec<&dyn AsContextField> = sub
            .fields
            .iter()
            .map(|f| f as &dyn AsContextField)
            .collect();
        let sub_ctx = SubTableContext {
            name: &sub.name,
            table: &sub.table,
            table_comment: &sub.table_comment,
            fk_field: &sub.fk_field,
            fields: &sub_field_refs,
        };
        return build_context(
            &input.name,
            &input.table,
            &input.package,
            &input.table_comment,
            &refs,
            Some(&sub_ctx),
            setup,
            git,
            user,
        );
    }
    build_context(
        &input.name,
        &input.table,
        &input.package,
        &input.table_comment,
        &refs,
        None,
        setup,
        git,
        user,
    )
}

fn insert_pk_builtins(map: &mut Map<String, Value>, fields: &[&dyn AsContextField]) {
    let (pk_field, pk_field_type, pk_field_pascal) = resolve_pk(fields);
    map.insert("pk_field".into(), Value::String(pk_field));
    map.insert("pk_field_type".into(), Value::String(pk_field_type));
    map.insert("pk_field_pascal".into(), Value::String(pk_field_pascal));
}

fn insert_sub_builtins(map: &mut Map<String, Value>, sub: Option<&SubTableContext<'_>>) {
    let Some(sub) = sub else {
        map.insert("is_sub".into(), Value::Bool(false));
        map.insert("sub_table".into(), Value::String(String::new()));
        map.insert("sub_table_comment".into(), Value::String(String::new()));
        map.insert("sub_fields".into(), Value::Array(vec![]));
        map.insert("sub_model".into(), Value::String(String::new()));
        map.insert("sub_model_snake".into(), Value::String(String::new()));
        map.insert("sub_model_pascal".into(), Value::String(String::new()));
        map.insert("sub_model_camel".into(), Value::String(String::new()));
        map.insert("sub_model_kebab".into(), Value::String(String::new()));
        map.insert("sub_model_fk".into(), Value::String(String::new()));
        map.insert("sub_model_fk_pascal".into(), Value::String(String::new()));
        return;
    };

    map.insert("is_sub".into(), Value::Bool(true));
    map.insert("sub_table".into(), Value::String(sub.table.to_string()));
    map.insert(
        "sub_table_comment".into(),
        Value::String(sub.table_comment.to_string()),
    );
    map.insert(
        "sub_fields".into(),
        Value::Array(sub.fields.iter().map(|f| field_to_json(*f)).collect()),
    );
    let sub_model = sub.name;
    map.insert("sub_model".into(), Value::String(sub_model.to_string()));
    map.insert(
        "sub_model_snake".into(),
        Value::String(sub_model.to_case(Case::Snake)),
    );
    map.insert(
        "sub_model_pascal".into(),
        Value::String(sub_model.to_case(Case::Pascal)),
    );
    map.insert(
        "sub_model_camel".into(),
        Value::String(sub_model.to_case(Case::Camel)),
    );
    map.insert(
        "sub_model_kebab".into(),
        Value::String(sub_model.to_case(Case::Kebab)),
    );
    let fk_camel = sub.fk_field.to_case(Case::Camel);
    map.insert("sub_model_fk".into(), Value::String(fk_camel.clone()));
    map.insert(
        "sub_model_fk_pascal".into(),
        Value::String(fk_camel.to_case(Case::Pascal)),
    );
}

/// Resolves primary-key builtins: camelCase name, raw type, PascalCase name.
fn resolve_pk(fields: &[&dyn AsContextField]) -> (String, String, String) {
    if let Some(pk) = fields.iter().copied().find(|f| f.is_pk()) {
        let camel = pk.name().to_case(Case::Camel);
        return (
            camel.clone(),
            pk.ty().to_string(),
            pk.name().to_case(Case::Pascal),
        );
    }
    ("id".into(), "Long".into(), "Id".into())
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
    m.insert("required".into(), Value::Bool(field.required()));
    m.insert("comment".into(), Value::String(field.comment().to_string()));
    m.insert(
        "length".into(),
        field
            .length()
            .map(|l| Value::Number(l.into()))
            .unwrap_or(Value::Null),
    );
    m.insert("unique".into(), Value::Bool(field.unique()));
    m.insert(
        "default".into(),
        field.default_value().cloned().unwrap_or(Value::Null),
    );
    for (k, v) in field.extra() {
        m.insert(k.clone(), v.clone());
    }
    Value::Object(m)
}
