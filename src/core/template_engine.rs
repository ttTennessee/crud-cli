//! Handlebars bootstrap with no HTML escaping (D-13, FOUND-08).

use convert_case::{Case, Casing};
use handlebars::{handlebars_helper, Handlebars, Helper, HelperResult, Output, RenderContext, RenderError, RenderErrorReason};
use std::collections::BTreeMap;
use std::sync::Arc;

use super::error::ErrorEnvelope;
use super::type_map::{self, Fallback};

handlebars_helper!(PascalCaseHelper: |v: str| v.to_case(Case::Pascal));
handlebars_helper!(SnakeCaseHelper: |v: str| v.to_case(Case::Snake));
handlebars_helper!(CamelCaseHelper: |v: str| v.to_case(Case::Camel));
handlebars_helper!(KebabCaseHelper: |v: str| v.to_case(Case::Kebab));

/// Captured by the `ty_map` helper closure for a single render.
#[derive(Debug, Clone)]
pub struct TypeMapBinding {
    pub bundle: Option<String>,
    pub map: Option<Arc<BTreeMap<String, String>>>,
    pub fallback: Fallback,
}

impl TypeMapBinding {
    /// No bundle, no map, passthrough — matches engines built via [`new_engine`].
    pub fn passthrough() -> Self {
        Self {
            bundle: None,
            map: None,
            fallback: Fallback::Passthrough,
        }
    }
}

/**
 * Returns a Handlebars registry with `handlebars::no_escape` registered exactly once (D-13).
 *
 * `ty_map` helper resolves via `Fallback::Passthrough`, so unknown types pass through unchanged.
 */
pub fn new_engine() -> Handlebars<'static> {
    new_engine_with_type_map(TypeMapBinding::passthrough())
}

/// Builds an engine where the `ty_map` helper is bound to the given map + fallback.
pub fn new_engine_with_type_map(binding: TypeMapBinding) -> Handlebars<'static> {
    let mut engine = Handlebars::new();
    engine.register_escape_fn(handlebars::no_escape);
    engine.register_helper("pascal_case", Box::new(PascalCaseHelper));
    engine.register_helper("snake_case", Box::new(SnakeCaseHelper));
    engine.register_helper("camel_case", Box::new(CamelCaseHelper));
    engine.register_helper("kebab_case", Box::new(KebabCaseHelper));
    engine.register_helper("ty_map", Box::new(make_ty_map_helper(binding)));
    engine
}

fn make_ty_map_helper(
    binding: TypeMapBinding,
) -> impl handlebars::HelperDef + Send + Sync + 'static {
    move |h: &Helper,
          _r: &Handlebars,
          _ctx: &handlebars::Context,
          _rc: &mut RenderContext,
          out: &mut dyn Output|
          -> HelperResult {
        let arg = h
            .param(0)
            .ok_or_else(|| RenderErrorReason::Other("ty_map: missing type argument".into()))?
            .value();
        let ty = arg
            .as_str()
            .ok_or_else(|| RenderErrorReason::Other(format!("ty_map: expected string, got {arg}")))?;

        let resolved = type_map::resolve(
            binding.bundle.as_deref(),
            binding.map.as_deref(),
            ty,
            &binding.fallback,
        )
        .map_err(|env| RenderError::from(RenderErrorReason::Other(env.msg.clone())))?;
        out.write(&resolved)?;
        Ok(())
    }
}

/**
 * Renders `template` with `data` using the no-escape engine.
 *
 * @param template - Handlebars template source
 * @param data - JSON context
 */
pub fn render_template(
    template: &str,
    data: &serde_json::Value,
) -> Result<String, ErrorEnvelope> {
    render_template_with_type_map(template, data, TypeMapBinding::passthrough())
}

/// Renders `template` with a `ty_map` helper bound to the given type map + fallback.
pub fn render_template_with_type_map(
    template: &str,
    data: &serde_json::Value,
    binding: TypeMapBinding,
) -> Result<String, ErrorEnvelope> {
    let engine = new_engine_with_type_map(binding);
    engine
        .render_template(template, data)
        .map_err(|e| ErrorEnvelope::template_error(format!("template render failed: {e}")))
}

#[cfg(test)]
mod case_helper_tests {
    #![allow(clippy::expect_used)]
    use super::*;

    #[test]
    fn case_helpers_render() {
        let engine = new_engine();
        let tpl = "{{pascal_case x}}|{{snake_case x}}|{{camel_case x}}|{{kebab_case x}}";
        let out = engine
            .render_template(tpl, &serde_json::json!({ "x": "hello_world" }))
            .expect("render");
        assert_eq!(out, "HelloWorld|hello_world|helloWorld|hello-world");
    }

    #[test]
    fn no_escape_list_generic() {
        let engine = new_engine();
        let out = engine
            .render_template("<List<{{x}}>>", &serde_json::json!({ "x": "T" }))
            .expect("render");
        assert_eq!(out, "<List<T>>");
    }

    #[test]
    fn ty_map_passthrough_when_no_map() {
        let out = render_template(
            "{{ty_map ty}}",
            &serde_json::json!({ "ty": "Integer" }),
        )
        .expect("render");
        assert_eq!(out, "Integer");
    }

    #[test]
    fn ty_map_resolves_via_bundle_map() {
        let mut m = BTreeMap::new();
        m.insert("int".into(), "number".into());
        m.insert("string".into(), "string".into());
        let binding = TypeMapBinding {
            bundle: Some("ts".into()),
            map: Some(Arc::new(m)),
            fallback: Fallback::Passthrough,
        };
        let out = render_template_with_type_map(
            "{{ty_map a}}|{{ty_map b}}|{{ty_map c}}",
            &serde_json::json!({ "a": "int", "b": "string", "c": "CustomType" }),
            binding,
        )
        .expect("render");
        // CustomType not in map → passthrough.
        assert_eq!(out, "number|string|CustomType");
    }

    #[test]
    fn ty_map_error_fallback_aborts_render() {
        let binding = TypeMapBinding {
            bundle: Some("ts".into()),
            map: Some(Arc::new(BTreeMap::new())),
            fallback: Fallback::Error,
        };
        let err = render_template_with_type_map(
            "{{ty_map ty}}",
            &serde_json::json!({ "ty": "Unknown" }),
            binding,
        )
        .expect_err("err");
        assert!(format!("{err:?}").contains("Unknown"));
    }

    #[test]
    fn ty_map_literal_fallback() {
        let binding = TypeMapBinding {
            bundle: Some("ts".into()),
            map: Some(Arc::new(BTreeMap::new())),
            fallback: Fallback::Literal("any".into()),
        };
        let out = render_template_with_type_map(
            "{{ty_map ty}}",
            &serde_json::json!({ "ty": "Unknown" }),
            binding,
        )
        .expect("render");
        assert_eq!(out, "any");
    }
}
