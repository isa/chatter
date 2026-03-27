use crate::cli::GlobalArgs;

pub fn run(_global: &GlobalArgs) -> anyhow::Result<()> {
    println!("Doctor command will be implemented with PyO3 bridge.");
    Ok(())
}
