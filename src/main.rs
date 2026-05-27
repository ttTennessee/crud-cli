//! Binary entry — panic hook and global CLI flags (`cli` feature).

use crud_cli::cli::{
    exit_with_envelope, init_agent_mode, panic_hook_handler, try_parse_cli_or_help, Commands,
};
use crud_cli::cli::setup_wizard::run_interactive_wizard;
fn main() {
    std::panic::set_hook(Box::new(panic_hook_handler));

    let cli = match try_parse_cli_or_help(std::env::args()) {
        Ok(Some(c)) => c,
        Ok(None) => return,
        Err(envelope) => std::process::exit(exit_with_envelope(&envelope)),
    };

    init_agent_mode(if cli.agent { Some(true) } else { None });

    let code = match cli.command {
        None => 0,
        Some(Commands::Setup(setup)) => run_setup(setup),
    };
    std::process::exit(code);
}

fn run_setup(setup: crud_cli::cli::SetupArgs) -> i32 {
    let result = if setup.is_non_interactive() {
        setup.to_setup_config()
    } else {
        run_interactive_wizard()
    };

    match result {
        Ok(cfg) => {
            match cfg.to_toml_pretty() {
                Ok(_toml) => {
                    // File write lands in plan 01-03+; contract path validates serialization.
                    crud_cli::cli::emit_success(None);
                    0
                }
                Err(envelope) => exit_with_envelope(&envelope),
            }
        }
        Err(envelope) => exit_with_envelope(&envelope),
    }
}
