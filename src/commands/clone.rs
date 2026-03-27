use owo_colors::{OwoColorize, Stream, Style};

use crate::cli::{CloneArgs, GlobalArgs};

pub fn run(_args: CloneArgs, _global: &GlobalArgs) -> anyhow::Result<()> {
    let style = Style::new().yellow().bold();
    eprintln!(
        "{} Clone is not yet implemented.",
        "Note:".if_supports_color(Stream::Stderr, |t| t.style(style))
    );
    eprintln!("This feature is planned for Phase 2.");
    Ok(())
}
