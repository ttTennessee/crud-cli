//! Binary entry — panic hook and global CLI flags (`cli` feature).

use crud_cli::cli::{
    exit_with_envelope, init_agent_mode, init_locale, panic_hook_handler, run_gen, run_setup,
    run_validate, try_parse_cli_or_help, Commands,
};

fn main() {
    std::panic::set_hook(Box::new(panic_hook_handler));

    let cli = match try_parse_cli_or_help(std::env::args()) {
        Ok(Some(c)) => c,
        Ok(None) => return,
        Err(envelope) => std::process::exit(exit_with_envelope(&envelope)),
    };

    init_agent_mode(if cli.agent { Some(true) } else { None });
    init_locale();

    let code = match cli.command {
        None => 0,
        Some(Commands::Setup(setup)) => run_setup(setup),
        Some(Commands::Gen(args)) => run_gen(args),
        Some(Commands::Validate(args)) => run_validate(args),
    };
    std::process::exit(code);
}
