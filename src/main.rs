mod bridge;
mod cli;
mod commands;
mod ui;

use clap::Parser;
use owo_colors::{OwoColorize, Stream};

use cli::{Cli, Commands};

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    // Commands that need Python: ensure venv is ready first
    if needs_python(&cli.command) {
        ensure_venv(&cli)?;
    }

    match cli.command {
        Commands::Design(args) => commands::design::run(args, &cli.global),
        Commands::Clone(args) => commands::clone::run(args, &cli.global),
        Commands::Generate(args) => commands::generate::run(args, &cli.global),
        Commands::Profiles { command } => commands::profiles::run(command, &cli.global),
        Commands::Model { command } => commands::model::run(command, &cli.global),
        Commands::Doctor => commands::doctor::run(&cli.global),
    }
}

/// Check if a command requires the Python venv to be set up.
fn needs_python(command: &Commands) -> bool {
    match command {
        // These commands call into Python via PyO3
        Commands::Model { .. } | Commands::Doctor | Commands::Design(_) | Commands::Clone(_) | Commands::Generate(_) => true,
        // Profile management is pure Rust (JSON files)
        Commands::Profiles { .. } => false,
    }
}

/// Ensure the managed Python venv exists and is configured.
/// On first run, creates the venv and installs dependencies with progress feedback.
fn ensure_venv(cli: &Cli) -> anyhow::Result<()> {
    if bridge::is_venv_ready() {
        // Venv exists and has qwen-tts — just configure PYTHONPATH
        bridge::configure_python_for_venv()?;
        return Ok(());
    }

    // First run — need to set up the environment
    let note = "Note:"
        .if_supports_color(Stream::Stderr, |t| t.yellow().to_string())
        .to_string();
    eprintln!("{note} Setting up Chatter environment (first run only)...\n");

    let spinner = ui::create_spinner("Creating Python environment and installing dependencies");

    match bridge::create_venv() {
        Ok(venv_path) => {
            spinner.finish_with_message("Environment ready");
            if cli.global.verbose {
                eprintln!("  Venv location: {}", venv_path.display());
            }
            eprintln!();
            // Now configure PYTHONPATH for the newly created venv
            bridge::configure_python_for_venv()?;
            Ok(())
        }
        Err(e) => {
            spinner.abandon_with_message("Environment setup failed");
            Err(anyhow::anyhow!(
                "Failed to set up Python environment: {e}\n\n\
                 Ensure Python 3.12+ is installed (brew install python@3.12) and try again.\n\
                 Run `chatter doctor` for detailed diagnostics."
            ))
        }
    }
}
