//! CLI subcommand handlers.

#[cfg(feature = "cli")]
pub mod gen;
#[cfg(feature = "mcp")]
pub mod mcp;
#[cfg(feature = "cli")]
pub mod setup;
#[cfg(feature = "cli")]
pub mod template;
#[cfg(feature = "cli")]
pub mod validate;

#[cfg(feature = "cli")]
pub use gen::run_gen;
#[cfg(feature = "mcp")]
pub use mcp::run_mcp;
#[cfg(feature = "cli")]
pub use setup::run_setup;
#[cfg(feature = "cli")]
pub use template::run_template;
#[cfg(feature = "cli")]
pub use validate::run_validate;
