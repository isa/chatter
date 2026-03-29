use std::io::Read;
use std::path::Path;

use anyhow::Context;

/// Extract text from a DOCX file.
///
/// DOCX files are ZIP archives. The main text lives in `word/document.xml`.
/// We parse the XML to extract text from `<w:t>` elements, inserting paragraph
/// breaks between `<w:p>` elements for natural chunking.
pub fn extract(path: &Path) -> anyhow::Result<String> {
    let file = std::fs::File::open(path).context("Failed to open DOCX file")?;
    let mut archive = zip::ZipArchive::new(file).context("Failed to read DOCX archive")?;

    let mut xml = String::new();
    archive
        .by_name("word/document.xml")
        .context("DOCX file missing word/document.xml — is this a valid Word document?")?
        .read_to_string(&mut xml)
        .context("Failed to read document.xml")?;

    Ok(extract_text_from_xml(&xml))
}

/// Parse Word XML and extract plain text.
///
/// Walks the XML character by character looking for `<w:t>` content and
/// `<w:p>` paragraph boundaries. This avoids pulling in a full XML parser.
fn extract_text_from_xml(xml: &str) -> String {
    let mut result = String::new();
    let mut in_text_element = false;
    let mut current_tag = String::new();
    let mut in_tag = false;
    let mut paragraph_has_text = false;

    for ch in xml.chars() {
        if ch == '<' {
            in_tag = true;
            in_text_element = false;
            current_tag.clear();
            continue;
        }

        if ch == '>' {
            in_tag = false;
            let tag = current_tag.trim();
            if tag == "w:t" || tag.starts_with("w:t ") {
                in_text_element = true;
            } else if tag == "/w:t" {
                in_text_element = false;
            } else if tag == "w:p" || tag.starts_with("w:p ") {
                // New paragraph — add double newline if previous paragraph had text
                if paragraph_has_text {
                    result.push_str("\n\n");
                }
                paragraph_has_text = false;
            } else if tag == "w:br" || tag == "w:br/" || tag.starts_with("w:br ") {
                result.push('\n');
            }
            current_tag.clear();
            continue;
        }

        if in_tag {
            current_tag.push(ch);
        } else if in_text_element {
            result.push(ch);
            paragraph_has_text = true;
        }
    }

    // Clean up excessive whitespace while preserving paragraph breaks
    let cleaned: Vec<&str> = result
        .split("\n\n")
        .map(|para| para.trim())
        .filter(|para| !para.is_empty())
        .collect();

    cleaned.join("\n\n")
}
