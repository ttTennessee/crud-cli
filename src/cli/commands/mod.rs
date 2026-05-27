//! CLI subcommand handlers.

#[cfg(feature = "cli")]
pub mod setup;

#[cfg(feature = "cli")]
pub use setup::run_setup;
