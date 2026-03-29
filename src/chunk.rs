/// Maximum characters per chunk before sub-splitting at sentence boundaries.
/// Qwen3-TTS has a 4096 token limit at 12.5 Hz codec rate. With ~6 codec tokens
/// per text token and overhead from ref_audio prefill, ~500 chars is a safe limit.
const MAX_CHUNK_CHARS: usize = 500;

/// Minimum characters for a chunk to be synthesizable.
/// Chunks shorter than this get merged with their neighbor.
const MIN_CHUNK_CHARS: usize = 20;

/// Preprocess text to insert pause markers for more natural TTS pacing.
///
/// Inserts `...` after sentence-ending punctuation and `..` after clause
/// punctuation. Does NOT insert newlines — chunking is handled separately.
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
/// Empty and too-short chunks are merged with neighbors. Very long paragraphs
/// are sub-split at sentence boundaries.
pub fn chunk_by_paragraph(text: &str) -> Vec<String> {
    let raw_chunks: Vec<String> = text
        .split("\n\n")
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .flat_map(|chunk| {
            if chunk.len() > MAX_CHUNK_CHARS {
                split_long_paragraph(&chunk)
            } else {
                vec![chunk]
            }
        })
        .collect();

    // Merge chunks that are too short with their neighbors
    merge_short_chunks(raw_chunks)
}

/// Merge chunks shorter than MIN_CHUNK_CHARS with adjacent chunks.
fn merge_short_chunks(chunks: Vec<String>) -> Vec<String> {
    if chunks.is_empty() {
        return chunks;
    }

    let mut merged: Vec<String> = Vec::with_capacity(chunks.len());

    for chunk in chunks {
        if chunk.len() < MIN_CHUNK_CHARS {
            // Append to previous chunk if one exists
            if let Some(prev) = merged.last_mut() {
                prev.push(' ');
                prev.push_str(&chunk);
            } else {
                merged.push(chunk);
            }
        } else {
            // Check if the last merged chunk is too short — absorb this into it
            if let Some(prev) = merged.last_mut() {
                if prev.len() < MIN_CHUNK_CHARS {
                    prev.push(' ');
                    prev.push_str(&chunk);
                    continue;
                }
            }
            merged.push(chunk);
        }
    }

    // Final pass: if the last chunk is still too short, merge it backward
    if merged.len() >= 2 {
        let last = merged.last().unwrap();
        if last.len() < MIN_CHUNK_CHARS {
            let last = merged.pop().unwrap();
            merged.last_mut().unwrap().push(' ');
            merged.last_mut().unwrap().push_str(&last);
        }
    }

    // Filter out anything that's only punctuation/whitespace
    merged
        .into_iter()
        .filter(|c| c.chars().any(|ch| ch.is_alphanumeric()))
        .collect()
}

/// Split a long paragraph at sentence boundaries.
fn split_long_paragraph(text: &str) -> Vec<String> {
    let bytes = text.as_bytes();
    let mut boundaries = Vec::new();

    for i in 0..bytes.len().saturating_sub(2) {
        if bytes[i] == b'.' && bytes[i + 1] == b' ' {
            let next = bytes[i + 2];
            if next.is_ascii_uppercase() || next == b'\n' {
                boundaries.push(i + 2);
            }
        }
    }

    if boundaries.is_empty() {
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

    let remaining = text[start..].trim().to_string();
    if !remaining.is_empty() {
        if remaining.len() > MAX_CHUNK_CHARS {
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
