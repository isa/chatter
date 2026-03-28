mod markdown;
mod pdf;
mod txt;

use std::path::Path;

/// Supported file formats for text extraction.
pub enum FileFormat {
    Txt,
    Markdown,
    Pdf,
    Unknown,
}

/// Detect file format from extension with content validation for PDF.
pub fn detect_format(path: &Path) -> FileFormat {
    match path.extension().and_then(|e| e.to_str()) {
        Some("txt") => FileFormat::Txt,
        Some("md" | "markdown") => FileFormat::Markdown,
        Some("pdf") => {
            // Content validation: check for %PDF magic bytes
            if let Ok(bytes) = std::fs::read(path) {
                if bytes.len() >= 4 && !bytes.starts_with(b"%PDF") {
                    eprintln!(
                        "Warning: File has .pdf extension but does not start with %PDF header."
                    );
                }
            }
            FileFormat::Pdf
        }
        _ => FileFormat::Unknown,
    }
}

/// Extract text from a file, dispatching to the appropriate extractor based on format.
pub fn extract_text(path: &Path) -> anyhow::Result<String> {
    let format = detect_format(path);
    match format {
        FileFormat::Txt => txt::extract(path),
        FileFormat::Markdown => markdown::extract(path),
        FileFormat::Pdf => pdf::extract(path),
        FileFormat::Unknown => {
            let ext = path
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("(none)");
            eprintln!(
                "Warning: Unrecognized file type '{}', attempting as plain text...",
                ext
            );
            txt::extract(path).map_err(|_| {
                anyhow::anyhow!(
                    "File contains binary or non-UTF-8 content and cannot be processed as text."
                )
            })
        }
    }
}
