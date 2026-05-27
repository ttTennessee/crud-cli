//! Binary entry — panic hook and global CLI flags (`cli` feature).

use clap::Parser;
use crud_cli::cli::{init_agent_mode, panic_hook_handler};

/// Top-level CLI (expanded in later plans).
#[derive(Parser, Debug)]
#[command(name = "crud-cli", version, about)]
struct Cli {
    /// Agent/machine mode: JSON errors on stderr, empty success stdout (D-05).
    #[arg(long, global = true)]
    agent: bool,
}

fn main() {
    std::panic::set_hook(Box::new(panic_hook_handler));
    let cli = Cli::parse();
    init_agent_mode(if cli.agent { Some(true) } else { None });
    // Setup command lands in plan 01-03+; contract-only stub exits 0.
    std::process::exit(0);
}
