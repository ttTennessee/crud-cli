//! `crud-cli mcp` — start the MCP server on stdio (`mcp` Cargo feature).

/**
 * Runs the MCP stdio server until the client disconnects.
 */
pub fn run_mcp() -> i32 {
    match crate::mcp::run_stdio_server_blocking() {
        Ok(()) => 0,
        Err(e) => {
            eprintln!("crud-cli mcp: {e}");
            1
        }
    }
}
