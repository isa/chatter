mod audio;
mod bridge;
mod chunk;
mod cli;
mod commands;
mod extract;
mod profile;
mod ui;

use clap::Parser;

use cli::{Cli, Commands};

// POSIX _exit: terminates immediately, skipping atexit handlers and Python finalization.
// std::process::exit still runs C atexit hooks where Python's resource_tracker segfaults.
unsafe extern "C" {
    fn _exit(status: i32) -> !;
}

fn main() {
    let result = run();
    match result {
        Ok(()) => unsafe { _exit(0) },
        Err(e) => {
            eprintln!("Error: {e:#}");
            unsafe { _exit(1) }
        }
    }
}

fn run() -> anyhow::Result<()> {
    let cli = Cli::parse();

    // Commands that need Python: ensure venv is found and configured
    if needs_python(&cli.command) {
        setup_python()?;
    }

    match cli.command {
        Commands::Design(args) => commands::design::run(args, &cli.global),
        Commands::Clone(args) => commands::clone::run(args, &cli.global),
        Commands::Generate(args) => commands::generate::run(args, &cli.global),
        Commands::Profiles { command } => commands::profiles::run(command, &cli.global),
        Commands::Model { command } => commands::model::run(command, &cli.global),
        Commands::Doctor(args) => {
            // Doctor configures Python if venv exists, but never errors if missing
            if bridge::is_venv_ready() {
                let _ = bridge::configure_python_for_venv();
            }
            commands::doctor::run(args, &cli.global)
        }
    }
}

/// Check if a command requires the Python venv.
fn needs_python(command: &Commands) -> bool {
    match command {
        Commands::Model { .. } | Commands::Design(_) | Commands::Clone(_) | Commands::Generate(_) => true,
        Commands::Doctor(_) | Commands::Profiles { .. } => false,
    }
}

/// Find and configure the Python venv. Errors if not found.
fn setup_python() -> anyhow::Result<()> {
    if !bridge::is_venv_ready() {
        anyhow::bail!(
            "Chatter environment not found.\n\n\
             If installed via Homebrew, try: brew reinstall chatter\n\
             For development, set CHATTER_VENV to your venv path.\n\
             Run `chatter doctor` for detailed diagnostics."
        );
    }

    bridge::configure_python_for_venv()?;
    bridge::ensure_bridge_installed()?;
    Ok(())
}
