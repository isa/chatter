mod bridge;
mod cli;
mod commands;

use clap::Parser;

use cli::{Cli, Commands};

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Design(args) => commands::design::run(args, &cli.global),
        Commands::Clone(args) => commands::clone::run(args, &cli.global),
        Commands::Generate(args) => commands::generate::run(args, &cli.global),
        Commands::Profiles { command } => commands::profiles::run(command, &cli.global),
        Commands::Model { command } => commands::model::run(command, &cli.global),
        Commands::Doctor => commands::doctor::run(&cli.global),
    }
}
