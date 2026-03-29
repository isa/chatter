use std::borrow::Cow;

/// Maximum characters per chunk before sub-splitting at sentence boundaries.
const MAX_CHUNK_CHARS: usize = 3000;

/// Preprocess text to insert pause markers for more natural TTS pacing.
///
/// - Adds `...` after sentence-ending punctuation (`.` `!` `?`) for longer pauses
/// - Adds `..` after commas, semicolons, and colons for short pauses
/// - Splits sentences onto separate lines so the model sees natural boundaries
pub fn add_pause_markers(text: &str) -> String {
    let mut result = String::with_capacity(text.len() * 2);
    let chars: Vec<char> = text.chars().collect();
    let len = chars.len();
    let mut i = 0;

    while i < len {
        let ch = chars[i];
        result.push(ch);

        match ch {
            '.' | '!' | '?' => {
                // Only add pauses if followed by a space (not abbreviations like "Dr.")
                if i + 1 < len && chars[i + 1] == ' ' {
                    result.push_str("...");
                    result.push('\n');
                    // Skip the space — the newline replaces it
                    i += 1;
                }
            }
            ',' | ';' | ':' => {
                if i + 1 < len && chars[i + 1] == ' ' {
                    result.push_str("..");
                }
            }
            _ => {}
        }
        i += 1;
    }

    result
}

/// Split text into chunks by paragraph breaks (double newlines).
///
/// Empty chunks are filtered. Very long paragraphs (>3000 chars) are
/// sub-split at sentence boundaries to keep TTS input within safe lengths.
pub fn chunk_by_paragraph(text: &str) -> Vec<String> {
    text.split("\n\n")
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .flat_map(|chunk| {
            if chunk.len() > MAX_CHUNK_CHARS {
                split_long_paragraph(&chunk)
            } else {
                vec![chunk]
            }
        })
        .collect()
}

/// Split a long paragraph at sentence boundaries.
///
/// Sentences are detected by ". " followed by an uppercase letter or newline.
/// If no sentence boundaries are found, splits at the nearest space to the
/// MAX_CHUNK_CHARS limit.
fn split_long_paragraph(text: &str) -> Vec<String> {
    // Find sentence boundary positions: ". " followed by uppercase or newline
    let bytes = text.as_bytes();
    let mut boundaries = Vec::new();

    for i in 0..bytes.len().saturating_sub(2) {
        if bytes[i] == b'.' && bytes[i + 1] == b' ' {
            // Check if next char is uppercase or newline
            let next = bytes[i + 2];
            if next.is_ascii_uppercase() || next == b'\n' {
                // Boundary is right after the period and space
                boundaries.push(i + 2);
            }
        }
    }

    if boundaries.is_empty() {
        // No sentence boundaries found -- split at nearest space to limit
        return split_at_spaces(text);
    }

    let mut chunks = Vec::new();
    let mut start = 0;

    for &boundary in &boundaries {
        if boundary - start >= MAX_CHUNK_CHARS {
            let chunk = text[start..boundary].trim().to_string();
            if !chunk.is_empty() {
                chunks.push(chunk);
            }
            start = boundary;
        }
    }

    // Remaining text
    let remaining = text[start..].trim().to_string();
    if !remaining.is_empty() {
        if remaining.len() > MAX_CHUNK_CHARS {
            // The remaining segment is still too long -- split at spaces
            chunks.extend(split_at_spaces(&remaining));
        } else {
            chunks.push(remaining);
        }
    }

    chunks
}

/// Fallback: split text at the nearest space to MAX_CHUNK_CHARS boundaries.
fn split_at_spaces(text: &str) -> Vec<String> {
    let mut chunks = Vec::new();
    let mut remaining = text;

    while remaining.len() > MAX_CHUNK_CHARS {
        // Find the last space within the limit
        let split_at = remaining[..MAX_CHUNK_CHARS]
            .rfind(' ')
            .unwrap_or(MAX_CHUNK_CHARS);

        let chunk = remaining[..split_at].trim().to_string();
        if !chunk.is_empty() {
            chunks.push(chunk);
        }
        remaining = remaining[split_at..].trim_start();
    }

    let last = remaining.trim().to_string();
    if !last.is_empty() {
        chunks.push(last);
    }

    chunks
}
