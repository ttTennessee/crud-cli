//! MCP server surface for `crud-cli mcp` (stdio transport via rmcp).

mod context;
mod convert;
mod resources;
mod server;
mod validate_logic;

pub use context::load_project_context;
pub use server::run_stdio_server;
pub use validate_logic::{describe_templates, validate_entity_json};

/**
 * Runs the MCP stdio server on a fresh tokio runtime (for sync `main` / CLI subcommands).
 */
pub fn run_stdio_server_blocking(explicit_path: Option<std::path::PathBuf>) -> Result<(), anyhow::Error> {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;
    rt.block_on(run_stdio_server(explicit_path))
}
