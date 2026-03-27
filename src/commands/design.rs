use owo_colors::{OwoColorize, Stream, Style};

use crate::cli::{DesignArgs, GlobalArgs};

pub fn run(_args: DesignArgs, _global: &GlobalArgs) -> anyhow::Result<()> {
    let style = Style::new().yellow().bold();
    eprintln!(
        "{} Design is not yet implemented.",
        "Note:".if_supports_color(Stream::Stderr, |t| t.style(style))
    );
    eprintln!("This feature is planned for Phase 2.");
    Ok(())
}
