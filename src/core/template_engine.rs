//! Handlebars bootstrap with no HTML escaping (D-13, FOUND-08).

use convert_case::{Case, Casing};
use handlebars::{handlebars_helper, Handlebars};

use super::error::ErrorEnvelope;

handlebars_helper!(PascalCaseHelper: |v: str| v.to_case(Case::Pascal));
handlebars_helper!(SnakeCaseHelper: |v: str| v.to_case(Case::Snake));
handlebars_helper!(CamelCaseHelper: |v: str| v.to_case(Case::Camel));
handlebars_helper!(KebabCaseHelper: |v: str| v.to_case(Case::Kebab));

/**
 * Returns a Handlebars registry with `handlebars::no_escape` registered exactly once (D-13).
 */
pub fn new_engine() -> Handlebars<'static> {
    let mut engine = Handlebars::new();
    engine.register_escape_fn(handlebars::no_escape);
    engine.register_helper("pascal_case", Box::new(PascalCaseHelper));
    engine.register_helper("snake_case", Box::new(SnakeCaseHelper));
    engine.register_helper("camel_case", Box::new(CamelCaseHelper));
    engine.register_helper("kebab_case", Box::new(KebabCaseHelper));
    engine
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
    let engine = new_engine();
    engine
        .render_template(template, data)
        .map_err(|e| ErrorEnvelope::template_error(format!("template render failed: {e}")))
}
