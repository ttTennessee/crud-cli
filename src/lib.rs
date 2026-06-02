//! `crud-cli` library — core contracts and optional CLI / MCP surface.

pub mod core;

#[cfg(feature = "cli")]
pub mod cli;

#[cfg(feature = "mcp")]
pub mod mcp;
