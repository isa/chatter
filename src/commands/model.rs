use crate::cli::{GlobalArgs, ModelCommands};

pub fn run(command: ModelCommands, _global: &GlobalArgs) -> anyhow::Result<()> {
    match command {
        ModelCommands::Download { size: _ } => {
            println!("Model download will be implemented with PyO3 bridge.");
        }
        ModelCommands::List => {
            println!("No models downloaded.");
        }
        ModelCommands::Remove { size: _ } => {
            println!("No models to remove.");
        }
    }
    Ok(())
}
