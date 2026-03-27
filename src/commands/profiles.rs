use crate::cli::{GlobalArgs, ProfilesCommands};

pub fn run(command: ProfilesCommands, _global: &GlobalArgs) -> anyhow::Result<()> {
    match command {
        ProfilesCommands::List => {
            println!("No voice profiles found.");
        }
        ProfilesCommands::Show { name } => {
            println!("Profile '{}' not found.", name);
        }
        ProfilesCommands::Delete { name, yes: _ } => {
            println!("Profile '{}' not found.", name);
        }
    }
    Ok(())
}
