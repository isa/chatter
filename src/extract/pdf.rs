use std::path::Path;

use anyhow::Context;
use pdf_extract::extract_text as pdf_extract_text;

/// Extract text from a PDF file with quality heuristics and image-page detection.
///
/// Validates PDF magic bytes, extracts text via pdf-extract, checks extraction
/// quality (short text, garbled characters), and inserts spoken placeholders
/// for pages that appear to contain mainly images or diagrams (D-09).
pub fn extract(path: &Path) -> anyhow::Result<String> {
    // Validate PDF magic bytes (D-01 content validation)
    let header_bytes = std::fs::read(path).context("Failed to read PDF file")?;
    if header_bytes.len() < 4 || !header_bytes.starts_with(b"%PDF") {
        anyhow::bail!("File does not appear to be a valid PDF (missing %PDF header)");
    }

    let text = pdf_extract_text(path).context("Failed to extract text from PDF")?;

    // Quality heuristic (D-08)
    let alpha_count = text.chars().filter(|c| c.is_alphanumeric()).count();
    let garbage_count = text.chars().filter(|c| *c == '\u{FFFD}').count();
    let garbage_ratio = garbage_count as f64 / text.len().max(1) as f64;

    if alpha_count < 50 {
        eprintln!(
            "Warning: Very little text extracted from PDF. The file may be image-based (scanned)."
        );
    }
    if garbage_ratio > 0.05 {
        eprintln!(
            "Warning: Extracted text contains unusual characters. Results may be imperfect."
        );
    }

    // Image-page detection heuristic (D-09)
    // pdf-extract inserts form-feed characters between pages
    let pages: Vec<&str> = text.split('\x0C').collect();

    if pages.len() <= 1 {
        // Single segment -- cannot analyze per-page, skip heuristic
        return Ok(text);
    }

    let mut result = String::new();
    let mut image_page_count = 0;

    for (i, page) in pages.iter().enumerate() {
        let page_alpha = page.chars().filter(|c| c.is_alphanumeric()).count();
        let is_not_empty = page.chars().any(|c| !c.is_alphanumeric() && !c.is_whitespace())
            || !page.trim().is_empty();

        if page_alpha < 20 && is_not_empty && !page.trim().is_empty() {
            // Likely an image-heavy or diagram page
            result.push_str("\n\nThis page appears to contain images or diagrams that cannot be extracted as text. See the original document for details.\n\n");
            image_page_count += 1;
        } else {
            result.push_str(page);
        }

        // Re-insert form-feed between pages (except after last)
        if i < pages.len() - 1 {
            result.push('\x0C');
        }
    }

    if image_page_count > 0 {
        eprintln!(
            "Warning: {} page(s) appear to contain mainly images/diagrams. \
             Spoken placeholders were inserted (per D-09). \
             Note: pdf-extract cannot detect inline images within text-heavy pages.",
            image_page_count
        );
    }

    Ok(result)
}
