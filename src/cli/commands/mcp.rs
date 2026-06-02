//! `crud-cli mcp` — start the MCP server on stdio (`mcp` Cargo feature).

use std::path::PathBuf;

use crate::cli::args::McpArgs;

/**
 * Resolves `--path` to an absolute directory when provided.
 */
fn resolve_explicit_path(raw: Option<&str>) -> Result<Option<PathBuf>, String> {
    let Some(path) = raw else {
        return Ok(None);
    };
    let p = PathBuf::from(path);
    let abs = std::fs::canonicalize(&p).map_err(|e| {
        format!("--path {}: {e}", p.display())
    })?;
    if !abs.is_dir() {
        return Err(format!("--path is not a directory: {}", abs.display()));
    }
    Ok(Some(abs))
}

/**
 * Runs the MCP stdio server until the client disconnects.
 */
pub fn run_mcp(args: McpArgs) -> i32 {
    let explicit = match resolve_explicit_path(args.path.as_deref()) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("crud-cli mcp: {e}");
            return 1;
        }
    };
    match crate::mcp::run_stdio_server_blocking(explicit) {
        Ok(()) => 0,
        Err(e) => {
            eprintln!("crud-cli mcp: {e}");
            1
        }
    }
}
