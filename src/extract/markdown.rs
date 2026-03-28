use std::path::Path;

use anyhow::Context;
use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};

/// Extract plain text from a Markdown file, stripping markup.
///
/// Code blocks are replaced with a spoken placeholder.
/// Images are replaced with a spoken placeholder.
/// Headers produce paragraph breaks (pauses).
/// Tables are skipped (complex layout, poor TTS).
pub fn extract(path: &Path) -> anyhow::Result<String> {
    let source = std::fs::read_to_string(path).context("Failed to read Markdown file")?;
    let parser = Parser::new_ext(&source, Options::empty());
    let mut output = String::new();
    let mut in_code_block = false;
    let mut in_table = false;

    for event in parser {
        match event {
            Event::Start(Tag::Heading { .. }) => {
                if !output.is_empty() {
                    output.push_str("\n\n");
                }
            }
            Event::End(TagEnd::Heading(_)) => {
                output.push_str("\n\n");
            }
            Event::Start(Tag::CodeBlock(_)) => {
                in_code_block = true;
                output.push_str(
                    "\n\nA code block appears here. See the original document for details.\n\n",
                );
            }
            Event::End(TagEnd::CodeBlock) => {
                in_code_block = false;
            }
            Event::Start(Tag::Image { .. }) => {
                output.push_str(
                    "\n\nAn image appears here. See the original document for details.\n\n",
                );
            }
            Event::Start(Tag::Table(_)) => {
                in_table = true;
            }
            Event::End(TagEnd::Table) => {
                in_table = false;
            }
            Event::Text(text) if !in_code_block && !in_table => {
                output.push_str(&text);
            }
            Event::Code(text) if !in_table => {
                output.push_str(&text);
            }
            Event::SoftBreak | Event::HardBreak if !in_code_block && !in_table => {
                output.push(' ');
            }
            Event::End(TagEnd::Paragraph) if !in_table => {
                output.push_str("\n\n");
            }
            Event::Rule => {
                output.push_str("\n\n");
            }
            _ => {}
        }
    }

    Ok(output)
}
