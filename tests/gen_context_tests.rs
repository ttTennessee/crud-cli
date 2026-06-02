//! `build_context` JSON shape tests (D-G04).
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use crud_cli::core::config::SetupConfig;
use crud_cli::core::config::SetupSelections;
use crud_cli::core::config::{Backend, Frontend};
use crud_cli::core::field_dsl::Field;
use crud_cli::core::gen_context::build_context_from_input;
use crud_cli::core::gen_input::GenInput;
use crud_cli::core::git_info::GitInfo;

#[test]
fn build_context_includes_model_and_field_case_keys() {
    let input = GenInput {
        name: "User".into(),
        table: "sys_user".into(),
        package: "com.acme.demo".into(),
        table_comment: String::new(),
        sub: None,
        fields: vec![
            Field {
                name: "id".into(),
                ty: "Long".into(),
                is_pk: true,
                nullable: false,
            },
            Field {
                name: "first_name".into(),
                ty: "String".into(),
                is_pk: false,
                nullable: false,
            },
        ],
    };
    let setup = SetupConfig::from_selections(SetupSelections {
        backend: Backend::None,
        frontend: Frontend::None,
        template: None,
    });
    let ctx = build_context_from_input(
        &input,
        &setup,
        &GitInfo::default(),
        &crud_cli::core::gen_context::UserIdentity::default(),
    )
    .expect("context");
    let obj = ctx.as_object().expect("object");

    assert_eq!(obj.get("model").and_then(|v| v.as_str()), Some("User"));
    assert_eq!(
        obj.get("model_pascal").and_then(|v| v.as_str()),
        Some("User")
    );
    assert_eq!(
        obj.get("model_snake").and_then(|v| v.as_str()),
        Some("user")
    );
    assert_eq!(
        obj.get("package_path").and_then(|v| v.as_str()),
        Some("com/acme/demo")
    );

    let fields = obj
        .get("fields")
        .and_then(|v| v.as_array())
        .expect("fields");
    let first = fields[1].as_object().expect("field obj");
    assert_eq!(
        first.get("name_pascal").and_then(|v| v.as_str()),
        Some("FirstName")
    );
    assert_eq!(
        first.get("name_camel").and_then(|v| v.as_str()),
        Some("firstName")
    );
    assert_eq!(
        first.get("name_kebab").and_then(|v| v.as_str()),
        Some("first-name")
    );
    assert_eq!(
        first.get("name_snake").and_then(|v| v.as_str()),
        Some("first_name")
    );
}

#[test]
fn build_context_includes_table_comment() {
    let input = GenInput {
        name: "User".into(),
        table: "sys_user".into(),
        package: "com.acme.demo".into(),
        table_comment: "系统用户".into(),
        sub: None,
        fields: vec![Field {
            name: "id".into(),
            ty: "Long".into(),
            is_pk: true,
            nullable: false,
        }],
    };
    let setup = SetupConfig::from_selections(SetupSelections {
        backend: Backend::None,
        frontend: Frontend::None,
        template: None,
    });
    let ctx = build_context_from_input(
        &input,
        &setup,
        &GitInfo::default(),
        &crud_cli::core::gen_context::UserIdentity::default(),
    )
    .expect("context");
    let obj = ctx.as_object().expect("object");
    assert_eq!(
        obj.get("table_comment").and_then(|v| v.as_str()),
        Some("系统用户")
    );
}

#[test]
fn build_context_surfaces_field_comment_length_unique_default_from_json_spec() {
    use crud_cli::core::gen_context::{build_context, AsContextField};
    use crud_cli::core::gen_input::FieldSpec;

    let id = FieldSpec {
        name: "id".into(),
        ty: "Long".into(),
        is_pk: true,
        nullable: false,
        required: false,
        length: None,
        unique: false,
        default: None,
        comment: "主键".into(),
        extra: serde_json::Map::new(),
    };
    let email = FieldSpec {
        name: "email".into(),
        ty: "String".into(),
        is_pk: false,
        nullable: false,
        required: false,
        length: Some(128),
        unique: true,
        default: Some(serde_json::json!("n/a")),
        comment: "邮箱".into(),
        extra: serde_json::Map::new(),
    };
    let refs: Vec<&dyn AsContextField> = vec![&id, &email];
    let setup = SetupConfig::from_selections(SetupSelections {
        backend: Backend::None,
        frontend: Frontend::None,
        template: None,
    });
    let ctx = build_context(
        "User",
        "sys_user",
        "com.acme.demo",
        "",
        &refs,
        None,
        &setup,
        &GitInfo::default(),
        &crud_cli::core::gen_context::UserIdentity::default(),
    )
    .expect("context");
    let obj = ctx.as_object().expect("object");

    let fields = obj.get("fields").and_then(|v| v.as_array()).expect("fields");
    let f0 = fields[0].as_object().expect("f0");
    assert_eq!(f0.get("comment").and_then(|v| v.as_str()), Some("主键"));
    assert_eq!(f0.get("length"), Some(&serde_json::Value::Null));
    assert_eq!(f0.get("unique").and_then(|v| v.as_bool()), Some(false));

    let f1 = fields[1].as_object().expect("f1");
    assert_eq!(f1.get("comment").and_then(|v| v.as_str()), Some("邮箱"));
    assert_eq!(f1.get("length").and_then(|v| v.as_u64()), Some(128));
    assert_eq!(f1.get("unique").and_then(|v| v.as_bool()), Some(true));
    assert_eq!(f1.get("default").and_then(|v| v.as_str()), Some("n/a"));
}

#[test]
fn build_context_derives_pk_and_sub_builtins() {
    use crud_cli::core::gen_context::{build_context, AsContextField, SubTableContext};
    use crud_cli::core::gen_input::FieldSpec;

    let id = FieldSpec {
        name: "order_id".into(),
        ty: "Long".into(),
        is_pk: true,
        nullable: false,
        required: false,
        length: None,
        unique: false,
        default: None,
        comment: "主键".into(),
        extra: serde_json::Map::new(),
    };
    let item_id = FieldSpec {
        name: "item_id".into(),
        ty: "Long".into(),
        is_pk: true,
        nullable: false,
        required: false,
        length: None,
        unique: false,
        default: None,
        comment: "明细主键".into(),
        extra: serde_json::Map::new(),
    };
    let order_fk = FieldSpec {
        name: "order_id".into(),
        ty: "Long".into(),
        is_pk: false,
        nullable: false,
        required: false,
        length: None,
        unique: false,
        default: None,
        comment: "订单外键".into(),
        extra: serde_json::Map::new(),
    };
    let master_refs: Vec<&dyn AsContextField> = vec![&id];
    let sub_refs: Vec<&dyn AsContextField> = vec![&item_id, &order_fk];
    let sub_ctx = SubTableContext {
        name: "OrderItem",
        table: "biz_order_item",
        table_comment: "订单明细",
        fk_field: "order_id",
        fields: &sub_refs,
    };
    let setup = SetupConfig::from_selections(SetupSelections {
        backend: Backend::None,
        frontend: Frontend::None,
        template: None,
    });
    let ctx = build_context(
        "Order",
        "biz_order",
        "com.acme.demo",
        "",
        &master_refs,
        Some(&sub_ctx),
        &setup,
        &GitInfo::default(),
        &crud_cli::core::gen_context::UserIdentity::default(),
    )
    .expect("context");
    let obj = ctx.as_object().expect("object");

    assert_eq!(obj.get("pk_field").and_then(|v| v.as_str()), Some("orderId"));
    assert_eq!(
        obj.get("pk_field_type").and_then(|v| v.as_str()),
        Some("Long")
    );
    assert_eq!(
        obj.get("pk_field_pascal").and_then(|v| v.as_str()),
        Some("OrderId")
    );
    assert_eq!(obj.get("is_sub").and_then(|v| v.as_bool()), Some(true));
    assert_eq!(
        obj.get("sub_table").and_then(|v| v.as_str()),
        Some("biz_order_item")
    );
    assert_eq!(
        obj.get("sub_table_comment").and_then(|v| v.as_str()),
        Some("订单明细")
    );
    assert_eq!(
        obj.get("sub_model_pascal").and_then(|v| v.as_str()),
        Some("OrderItem")
    );
    assert_eq!(
        obj.get("sub_model_fk").and_then(|v| v.as_str()),
        Some("orderId")
    );
    let sub_fields = obj
        .get("sub_fields")
        .and_then(|v| v.as_array())
        .expect("sub_fields");
    assert_eq!(sub_fields.len(), 2);
}

#[test]
fn build_context_non_sub_pk_defaults_to_id() {
    use crud_cli::core::gen_context::{build_context, AsContextField};
    use crud_cli::core::gen_input::FieldSpec;

    let email = FieldSpec {
        name: "email".into(),
        ty: "String".into(),
        is_pk: false,
        nullable: false,
        required: false,
        length: None,
        unique: false,
        default: None,
        comment: String::new(),
        extra: serde_json::Map::new(),
    };
    let refs: Vec<&dyn AsContextField> = vec![&email];
    let setup = SetupConfig::from_selections(SetupSelections {
        backend: Backend::None,
        frontend: Frontend::None,
        template: None,
    });
    let ctx = build_context(
        "User",
        "sys_user",
        "com.acme.demo",
        "",
        &refs,
        None,
        &setup,
        &GitInfo::default(),
        &crud_cli::core::gen_context::UserIdentity::default(),
    )
    .expect("context");
    let obj = ctx.as_object().expect("object");
    assert_eq!(obj.get("is_sub").and_then(|v| v.as_bool()), Some(false));
    assert_eq!(obj.get("pk_field").and_then(|v| v.as_str()), Some("id"));
    assert_eq!(
        obj.get("pk_field_type").and_then(|v| v.as_str()),
        Some("Long")
    );
}

#[test]
fn gen_report_serializes() {
    let report = crud_cli::core::gen_report::GenReport::default();
    let json = serde_json::to_string(&report).expect("serialize");
    assert!(json.contains("\"written\""));
}
