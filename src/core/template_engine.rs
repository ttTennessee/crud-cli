//! Handlebars bootstrap with no HTML escaping (D-13, FOUND-08).

use handlebars::Handlebars;

use super::error::ErrorEnvelope;

/**
 * Returns a Handlebars registry with `handlebars::no_escape` registered exactly once (D-13).
 */
pub fn new_engine() -> Handlebars<'static> {
    let mut engine = Handlebars::new();
    engine.register_escape_fn(handlebars::no_escape);
    engine
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
