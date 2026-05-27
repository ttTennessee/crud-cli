//! CLI subcommand handlers.

#[cfg(feature = "cli")]
pub mod gen;
#[cfg(feature = "cli")]
pub mod setup;
#[cfg(feature = "cli")]
pub mod validate;

#[cfg(feature = "cli")]
pub use gen::run_gen;
#[cfg(feature = "cli")]
pub use setup::run_setup;
#[cfg(feature = "cli")]
pub use validate::run_validate;
