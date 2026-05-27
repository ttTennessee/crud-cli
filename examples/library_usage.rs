//! Core-only library consumption contract (FOUND-03).
//!
//! Run: `cargo run --example library_usage --no-default-features`

use crud_cli::core::template_engine;

fn main() {
    let data = serde_json::json!({ "label": "<raw>&" });
    let rendered = template_engine::render_template("{{label}}", &data).unwrap_or_else(|e| {
        eprintln!("render failed: {}", e.msg);
        std::process::exit(e.exit_code);
    });
    assert_eq!(rendered, "<raw>&");
    println!("library_usage ok");
}
