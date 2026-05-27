//! Handlebars JSON context builder (D-G04, D-G06).

use convert_case::{Case, Casing};
use serde_json::{Map, Value};

use super::config::SetupConfig;
use super::field_dsl::Field;
use super::gen_input::GenInput;
use super::git_info::GitInfo;

/**
 * Builds the top-level render context for templates.
 *
 * `_setup` is reserved for Plan 02 `[variables]` merge; Wave 1 ignores it.
 */
pub fn build_context(
    input: &GenInput,
    _setup: &SetupConfig,
    git: &GitInfo,
) -> Value {
    let model = input.name.as_str();
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
    map.insert("table".into(), Value::String(input.table.clone()));
    map.insert("package".into(), Value::String(input.package.clone()));
    map.insert(
        "package_path".into(),
        Value::String(input.package.replace('.', "/")),
    );
    map.insert(
        "fields".into(),
        Value::Array(input.fields.iter().map(field_to_json).collect()),
    );
    map.insert("git_user_name".into(), Value::String(git.user_name.clone()));
    map.insert(
        "git_user_email".into(),
        Value::String(git.user_email.clone()),
    );
    Value::Object(map)
}

fn field_to_json(field: &Field) -> Value {
    let name = field.name.as_str();
    let mut m = Map::new();
    m.insert("name".into(), Value::String(field.name.clone()));
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
    m.insert("type".into(), Value::String(field.ty.clone()));
    m.insert("is_pk".into(), Value::Bool(field.is_pk));
    m.insert("nullable".into(), Value::Bool(field.nullable));
    Value::Object(m)
}
