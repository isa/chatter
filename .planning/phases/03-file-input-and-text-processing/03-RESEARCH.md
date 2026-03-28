# Phase 3: File Input and Text Processing - Research

**Researched:** 2026-03-28
**Domain:** File I/O, text extraction (TXT/Markdown/PDF), audio chunking and concatenation
**Confidence:** HIGH

## Summary

Phase 3 extends the existing `chatter generate` command to accept file input (`--file <path>`) supporting TXT, Markdown, and PDF formats. The core work involves: (1) a file detection and reading layer, (2) format-specific text extraction (trivial for TXT, event-based stripping for Markdown, library-based extraction for PDF), (3) paragraph-based chunking with per-chunk synthesis, (4) PCM sample concatenation before final MP3 encoding, and (5) a `--split` flag for per-chunk output files.

The technology stack is already decided in CLAUDE.md: `pulldown-cmark 0.13.3` for Markdown and `pdf-extract 0.10.0` for PDF. Both are confirmed current on crates.io. The existing audio pipeline (`samples_f32_to_i16` + `encode_wav_to_mp3`) and inference bridge (`generate_speech`) are reused directly -- the new code calls `generate_speech` per chunk, collects `Vec<f32>` samples, concatenates them (with optional silence gaps), then encodes once to MP3.

**Primary recommendation:** Build a `src/extract/` module with a trait-based extractor pattern (one impl per format), a chunker that splits extracted text by paragraph, and wire it into the existing generate command at the `--file` bail point (line 41-53 of `generate.rs`).

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions
- **D-01:** Extension-based detection with content validation. Check file extension first (.txt, .md/.markdown, .pdf), then validate content (e.g., PDF magic bytes `%PDF`). Catches misnamed files.
- **D-02:** Unsupported file types (.docx, .html, etc.) are attempted as plain UTF-8 text with a warning. If the file contains binary/non-UTF-8 content, fail gracefully with a clear error.
- **D-03:** Chunk text by paragraph breaks (double newline). Each paragraph becomes a separate synthesis call. Audio from all chunks is concatenated into one MP3.
- **D-04:** Single output file by default. Add `--split` flag to produce numbered separate files per chunk (e.g., `output-001.mp3`, `output-002.mp3`, ...).
- **D-05:** Strip Markdown markup but preserve structure hints. Add pauses (extra newlines / silence) after headers and between sections so the audio flows naturally for structured documents.
- **D-06:** Code blocks are replaced with a spoken placeholder: "A code block appears here. See the original document for details." -- with pauses before and after. Code content is not read aloud.
- **D-07:** Images and diagrams in Markdown are replaced with a spoken placeholder: "An image appears here. See the original document for details." -- same pause treatment as code blocks.
- **D-08:** Best-effort extraction using `pdf-extract` crate. If extracted text looks short relative to page count or contains garbled characters (heuristic), warn the user but proceed with what was extracted.
- **D-09:** Images and diagrams in PDFs get the same spoken placeholder treatment as Markdown (D-07).
- **D-10:** Read as-is. UTF-8 encoding assumed. No processing beyond paragraph chunking.
- **D-11:** Multi-step progress display with distinct phases: "Reading file..." -> "Extracting text..." -> "Synthesizing (3/12)..." -> "Encoding MP3..."
- **D-12:** Synthesis step uses a bounded progress bar showing chunk progress: `[========>           ] 3/12 chunks`. Other steps (reading, extracting, encoding) use spinners consistent with Phase 1/2 style.

### Claude's Discretion
- Exact heuristics for PDF quality detection (garbled text, short extraction)
- Paragraph chunking edge cases (very long paragraphs, single-line documents)
- How silence/pauses are inserted between chunks and after headers (duration, method)
- `--split` flag naming convention for output files
- How `pulldown-cmark` events map to text extraction (which AST nodes to strip vs preserve)
- MP3 concatenation approach (concatenate PCM samples before encoding, or encode per-chunk and join)

### Deferred Ideas (OUT OF SCOPE)
None -- discussion stayed within phase scope.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| GEN-02 | User can generate speech from a TXT file path via `chatter generate --file <path>` | TXT extractor reads UTF-8, chunks by paragraph, feeds to existing generate pipeline |
| GEN-03 | User can generate speech from a Markdown file path (formatting stripped before synthesis) | pulldown-cmark event iterator strips markup, inserts placeholders for code/images, preserves text |
| GEN-04 | User can generate speech from a PDF file path (basic text extraction) | pdf-extract `extract_text` provides full text; heuristic quality check warns on poor extraction |
</phase_requirements>

## Project Constraints (from CLAUDE.md)

- **Tech stack**: Rust CLI with PyO3 for Python interop
- **Audio output**: MP3 (WAV-to-MP3 via mp3lame-encoder with LAME static linking)
- **No async**: Synchronous CLI, no tokio/async-std
- **Error handling**: anyhow for top-level, thiserror for library modules
- **Progress**: indicatif spinners/bars, owo-colors with NO_COLOR compliance
- **No new Python deps**: All file processing is pure Rust. Only synthesis uses Python bridge.

## Standard Stack

### Core (already in Cargo.toml)
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| pulldown-cmark | 0.13.3 | Markdown to plain text | CommonMark-compliant streaming parser. Event iterator enables selective text extraction without AST allocation. |
| pdf-extract | 0.10.0 | PDF text extraction | Purpose-built for text extraction. Simple `extract_text(path)` API. |
| hound | 3.5.1 | WAV reading (existing) | Already used in audio pipeline |
| mp3lame-encoder | 0.2.2 | MP3 encoding (existing) | Already used in audio pipeline |
| indicatif | 0.18.4 | Progress bars (existing) | Already used; extend with bounded `ProgressBar::new(total)` for chunk progress |

### New Dependencies to Add
| Library | Version | Purpose |
|---------|---------|---------|
| pulldown-cmark | 0.13.3 | Markdown parsing -- NOT yet in Cargo.toml |
| pdf-extract | 0.10.0 | PDF extraction -- NOT yet in Cargo.toml |

**Installation:**
```bash
cargo add pulldown-cmark@0.13.3
cargo add pdf-extract@0.10.0
```

## Architecture Patterns

### Recommended Project Structure
```
src/
├── extract/
│   ├── mod.rs          # ExtractedText type, detect_format(), extract_text() dispatcher
│   ├── txt.rs          # TXT: read UTF-8, return raw text
│   ├── markdown.rs     # Markdown: pulldown-cmark event stripping
│   └── pdf.rs          # PDF: pdf-extract wrapper with quality heuristics
├── chunk.rs            # Paragraph chunking logic (split on double newlines)
├── commands/
│   └── generate.rs     # Extended: file path branch, chunk loop, progress bar
├── audio/
│   └── mod.rs          # Extended: silence generation helper
└── ...existing...
```

### Pattern 1: Format Detection and Dispatch
**What:** Detect file format by extension + content validation, dispatch to appropriate extractor.
**When to use:** Entry point for `--file` processing.
**Example:**
```rust
// src/extract/mod.rs
pub enum FileFormat {
    Txt,
    Markdown,
    Pdf,
    Unknown, // fallback: attempt as UTF-8 text with warning
}

pub fn detect_format(path: &Path) -> FileFormat {
    match path.extension().and_then(|e| e.to_str()) {
        Some("txt") => FileFormat::Txt,
        Some("md" | "markdown") => FileFormat::Markdown,
        Some("pdf") => {
            // Validate PDF magic bytes
            if let Ok(bytes) = std::fs::read(path).map(|b| b.len() >= 4) {
                // Check for %PDF header
                FileFormat::Pdf
            } else {
                FileFormat::Pdf // attempt anyway
            }
        }
        _ => FileFormat::Unknown,
    }
}

pub fn extract_text(path: &Path) -> anyhow::Result<String> {
    let format = detect_format(path);
    match format {
        FileFormat::Txt => txt::extract(path),
        FileFormat::Markdown => markdown::extract(path),
        FileFormat::Pdf => pdf::extract(path),
        FileFormat::Unknown => {
            eprintln!("Warning: Unknown file type, attempting as plain text...");
            txt::extract(path) // Falls back to UTF-8 read
        }
    }
}
```

### Pattern 2: Markdown Event-Based Text Extraction
**What:** Iterate pulldown-cmark events, emit plain text with structure-aware placeholders and pause markers.
**When to use:** Processing `.md` files per D-05, D-06, D-07.
**Example:**
```rust
// src/extract/markdown.rs
use pulldown_cmark::{Event, Parser, Tag, TagEnd, Options};

pub fn extract(path: &Path) -> anyhow::Result<String> {
    let source = std::fs::read_to_string(path)?;
    let parser = Parser::new_ext(&source, Options::empty());
    let mut output = String::new();
    let mut in_code_block = false;

    for event in parser {
        match event {
            Event::Start(Tag::Heading { .. }) => {
                // Heading starts -- ensure paragraph break before
                if !output.is_empty() {
                    output.push_str("\n\n");
                }
            }
            Event::End(TagEnd::Heading(_)) => {
                // Add pause marker after heading
                output.push_str("\n\n");
            }
            Event::Start(Tag::CodeBlock(_)) => {
                in_code_block = true;
                output.push_str("\n\nA code block appears here. See the original document for details.\n\n");
            }
            Event::End(TagEnd::CodeBlock) => {
                in_code_block = false;
            }
            Event::Start(Tag::Image { .. }) => {
                output.push_str("\n\nAn image appears here. See the original document for details.\n\n");
            }
            Event::Text(text) if !in_code_block => {
                output.push_str(&text);
            }
            Event::SoftBreak | Event::HardBreak => {
                output.push(' ');
            }
            Event::End(TagEnd::Paragraph) => {
                output.push_str("\n\n");
            }
            _ => {} // Skip all other events
        }
    }
    Ok(output)
}
```

### Pattern 3: Paragraph Chunking
**What:** Split extracted text into chunks by double newlines. Handle edge cases (very long paragraphs, empty chunks).
**When to use:** After text extraction, before synthesis loop.
**Example:**
```rust
// src/chunk.rs
pub fn chunk_by_paragraph(text: &str) -> Vec<String> {
    text.split("\n\n")
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}
```

### Pattern 4: PCM Concatenation with Silence Gaps
**What:** Concatenate per-chunk audio samples with silence inserted between chunks and after structural elements.
**When to use:** After all chunks are synthesized, before MP3 encoding.
**Recommendation (Claude's Discretion):** Concatenate raw `Vec<f32>` PCM samples, then encode once to MP3. This avoids MP3 frame alignment issues from concatenating compressed streams.
**Example:**
```rust
/// Generate silence samples for a given duration.
/// sample_rate is typically 24000 for Qwen3-TTS.
fn silence_samples(duration_ms: u32, sample_rate: u32) -> Vec<f32> {
    let num_samples = (sample_rate * duration_ms) / 1000;
    vec![0.0f32; num_samples as usize]
}

/// Concatenate chunk audio with silence gaps.
fn concatenate_chunks(chunks: &[(Vec<f32>, u32)], gap_ms: u32) -> (Vec<f32>, u32) {
    let sample_rate = chunks[0].1; // All chunks share sample rate
    let gap = silence_samples(gap_ms, sample_rate);
    let mut combined = Vec::new();
    for (i, (samples, _)) in chunks.iter().enumerate() {
        combined.extend_from_slice(samples);
        if i < chunks.len() - 1 {
            combined.extend_from_slice(&gap);
        }
    }
    (combined, sample_rate)
}
```

### Pattern 5: Bounded Progress Bar for Chunk Synthesis
**What:** Replace spinner with bounded progress bar during synthesis loop.
**Example:**
```rust
use indicatif::{ProgressBar, ProgressStyle};

let pb = ProgressBar::new(chunks.len() as u64);
pb.set_style(
    ProgressStyle::with_template("{spinner:.cyan} Synthesizing ({pos}/{len}) [{bar:30}] ({elapsed})")
        .expect("valid template")
        .progress_chars("=>-"),
);
pb.enable_steady_tick(std::time::Duration::from_millis(100));

for (i, chunk) in chunks.iter().enumerate() {
    let (wav, sr) = inference::generate_speech(chunk, language_str, &profile_dir)?;
    audio_chunks.push((wav, sr));
    pb.inc(1);
}
pb.finish_and_clear();
```

### Anti-Patterns to Avoid
- **Encoding MP3 per chunk then concatenating MP3 files:** MP3 frames have headers and padding. Naive concatenation produces audible glitches at boundaries. Always concatenate raw PCM, encode once.
- **Reading entire PDF into String without quality check:** pdf-extract can return garbled text for scanned PDFs. Always run the quality heuristic before proceeding.
- **Using pulldown-cmark's HTML renderer:** We need plain text, not HTML. Use the event iterator directly.
- **Blocking on very large files without feedback:** Always show the spinner during file reading and text extraction, even if they are fast for most files.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Markdown parsing | Regex-based stripping | pulldown-cmark event iterator | Markdown has complex nesting; regex will miss edge cases (nested lists, indented code, etc.) |
| PDF text extraction | Custom PDF parser | pdf-extract `extract_text()` | PDF format is extraordinarily complex (fonts, encodings, layout). Even pdf-extract struggles with some files. |
| MP3 encoding | Custom encoder | mp3lame-encoder (existing) | Already in the project, works. |
| Progress bars | Manual terminal escape codes | indicatif (existing) | Handles terminal width, redrawing, elapsed time, all edge cases. |

**Key insight:** The text extraction libraries do the heavy lifting. The novel code in this phase is the glue: format detection, chunking, the synthesis loop, and PCM concatenation.

## Common Pitfalls

### Pitfall 1: PDF Extraction Returns Garbage for Scanned PDFs
**What goes wrong:** `pdf-extract` returns empty or garbled text for image-based (scanned) PDFs.
**Why it happens:** pdf-extract extracts text from the PDF text layer. Scanned PDFs have no text layer.
**How to avoid:** Implement quality heuristics per D-08: (1) compare extracted text length to page count (less than ~50 chars per page is suspicious), (2) check for high ratio of non-printable or replacement characters. Warn user but proceed.
**Warning signs:** Very short output, lots of `\u{FFFD}` replacement characters, or output that is all whitespace.

### Pitfall 2: Very Long Paragraphs Cause TTS Quality Degradation
**What goes wrong:** A single paragraph with 2000+ words produces poor audio quality or the model truncates silently.
**Why it happens:** TTS models have practical input length limits. Qwen3-TTS handles typical paragraphs well but may degrade on very long inputs.
**How to avoid:** If a chunk exceeds a threshold (recommendation: ~500 words / ~3000 characters), split it further at sentence boundaries (period + space). This is a Claude's Discretion item.
**Warning signs:** Audio cuts off abruptly or quality drops noticeably for long passages.

### Pitfall 3: Empty Chunks After Stripping
**What goes wrong:** Markdown files with mostly code blocks or images produce chunks that are only placeholder text or empty after stripping.
**Why it happens:** A file that is 90% code blocks will have very little spoken content.
**How to avoid:** Filter empty chunks after splitting. If the final text is very short relative to file size, inform the user (similar to PDF quality warning).
**Warning signs:** Unexpectedly short audio output for large files.

### Pitfall 4: Inconsistent Sample Rates Between Chunks
**What goes wrong:** Concatenation assumes all chunks have the same sample rate, but one call returns a different rate.
**Why it happens:** Unlikely with Qwen3-TTS (always 24000 Hz), but defensive coding prevents hard-to-debug audio artifacts.
**How to avoid:** Assert sample rate consistency across chunks. Use the first chunk's sample rate as reference and bail if any chunk differs.

### Pitfall 5: `--split` and `--output` Interaction
**What goes wrong:** User provides `--output result.mp3` with `--split`, and the naming convention is unclear.
**Why it happens:** Need a clear convention for how `--split` uses the output path.
**How to avoid:** If `--output` is given with `--split`, use it as a base name: `result-001.mp3`, `result-002.mp3`. If no `--output`, use the default timestamp pattern with chunk numbers.

## Code Examples

### PDF Extraction with Quality Heuristic
```rust
// src/extract/pdf.rs
use pdf_extract::extract_text as pdf_extract_text;

pub fn extract(path: &Path) -> anyhow::Result<String> {
    // Validate PDF magic bytes
    let header = std::fs::read(path)
        .map(|b| b.starts_with(b"%PDF"))
        .unwrap_or(false);
    if !header {
        anyhow::bail!("File does not appear to be a valid PDF (missing %PDF header)");
    }

    let text = pdf_extract_text(path)
        .context("Failed to extract text from PDF")?;

    // Quality heuristic: warn if extraction looks poor
    let char_count = text.chars().filter(|c| c.is_alphanumeric()).count();
    let garbage_ratio = text.chars().filter(|c| *c == '\u{FFFD}').count() as f64
        / text.len().max(1) as f64;

    if char_count < 50 {
        eprintln!("Warning: Very little text extracted from PDF. The file may be image-based (scanned).");
    } else if garbage_ratio > 0.05 {
        eprintln!("Warning: Extracted text contains unusual characters. Results may be imperfect.");
    }

    Ok(text)
}
```

### Full Generate Command Flow (Pseudocode)
```rust
// In generate.rs run(), replace the --file bail:
let text = match (&args.text, &args.file) {
    (Some(t), _) => t.clone(),
    (None, Some(file_path)) => {
        let spinner = ui::create_spinner("Reading file...");
        let raw = extract::extract_text(file_path)?;
        spinner.finish_and_clear();
        raw
    }
    (None, None) => anyhow::bail!("Provide text or --file"),
};

// Chunk the text
let chunks = chunk::chunk_by_paragraph(&text);
if chunks.is_empty() {
    anyhow::bail!("No text content found in file");
}

// Synthesize each chunk with progress
let pb = ProgressBar::new(chunks.len() as u64);
// ... configure style ...
let mut audio_parts: Vec<(Vec<f32>, u32)> = Vec::new();
for chunk in &chunks {
    let (wav, sr) = inference::generate_speech(chunk, language_str, &profile_dir)?;
    audio_parts.push((wav, sr));
    pb.inc(1);
}
pb.finish_and_clear();

// Concatenate or split
if args.split {
    for (i, (wav, sr)) in audio_parts.iter().enumerate() {
        let pcm = audio::samples_f32_to_i16(wav);
        let chunk_path = split_output_path(&output_path, i + 1);
        audio::encode_wav_to_mp3(&pcm, *sr, &chunk_path)?;
    }
} else {
    let (combined, sr) = concatenate_chunks(&audio_parts, 300); // 300ms gap
    let pcm = audio::samples_f32_to_i16(&combined);
    audio::encode_wav_to_mp3(&pcm, sr, &output_path)?;
}
```

## Discretion Recommendations

These are areas marked as Claude's Discretion. Research-informed recommendations:

### Silence Duration Between Chunks
**Recommendation:** 300ms silence between regular paragraphs, 500ms after headings. These values produce natural-sounding breaks in spoken content. Implemented as zero-valued f32 samples at the model's sample rate (24000 Hz): 300ms = 7200 samples, 500ms = 12000 samples.

### PDF Quality Heuristics
**Recommendation:** Two checks:
1. **Length check:** If total alphanumeric characters < 50 per page, warn about possible scanned PDF.
2. **Garbage check:** If ratio of Unicode replacement characters (`\u{FFFD}`) to total characters exceeds 5%, warn about encoding issues.
Both checks warn but do not block -- proceed with whatever text was extracted.

### Very Long Paragraph Handling
**Recommendation:** If a single chunk exceeds 3000 characters, split at sentence boundaries (`. ` followed by uppercase letter or newline). This keeps TTS input within safe lengths without losing coherence.

### Split File Naming
**Recommendation:** Given output path `foo.mp3`, split files are `foo-001.mp3`, `foo-002.mp3`, etc. Given default naming `profile-timestamp.mp3`, split files are `profile-timestamp-001.mp3`, etc. Zero-padded to 3 digits (supports up to 999 chunks).

### MP3 Concatenation Approach
**Recommendation:** Concatenate raw PCM f32 samples with silence gaps, then encode once to MP3. This is simpler and avoids MP3 frame boundary artifacts. The existing `encode_wav_to_mp3` function handles the final encoding unchanged.

### pulldown-cmark Event Mapping
**Recommendation:**
| Event | Action |
|-------|--------|
| `Text(t)` (not in code block) | Append text |
| `Code(t)` (inline code) | Append text as-is (spoken naturally) |
| `SoftBreak` / `HardBreak` | Append space |
| `Start(Heading{..})` | Ensure double newline before |
| `End(Heading)` | Append double newline (pause marker) |
| `Start(CodeBlock(..))` | Insert placeholder, set flag |
| `End(CodeBlock)` | Clear flag |
| `Start(Image{..})` | Insert placeholder |
| `Start(Paragraph)` | No action |
| `End(Paragraph)` | Append double newline |
| `Start/End(List/Item)` | No special action, text events carry content |
| `Start/End(BlockQuote)` | No special action |
| `Start/End(Table*)` | Skip table content (complex layout, poor TTS) |
| `Rule` | Append double newline (section break) |
| Everything else | Skip |

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| pulldown-cmark 0.11 with `Tag` in End events | 0.13 uses `TagEnd` enum (no data) in End events | 0.12 (2024) | Must match on `TagEnd::Heading(_)` not `Tag::Heading{..}` for end events |
| pdf-extract < 0.8 API | 0.10.0 with `extract_text(path)` returning `Result<String>` | 2025 | Stable API, no recent breaking changes |

**Note on pulldown-cmark 0.13 API change:** The `Event::End` variant now contains `TagEnd` (a simpler enum without payload) instead of `Tag`. This means you cannot inspect heading level in `End` events -- only in `Start` events. Code examples above reflect this.

## Open Questions

1. **Qwen3-TTS maximum input length**
   - What we know: The model works well with typical paragraphs (100-300 words). Training data suggests reasonable performance up to ~500 words.
   - What's unclear: Exact hard limit or degradation point for input text length.
   - Recommendation: Use 3000-character chunk limit as a safe default. Monitor audio quality in testing.

2. **PDF page count detection for heuristic**
   - What we know: `pdf-extract` has `extract_text_by_pages` which returns `Vec<String>` (one per page), giving us page count implicitly.
   - What's unclear: Whether calling `extract_text_by_pages` instead of `extract_text` has different extraction quality.
   - Recommendation: Use `extract_text_by_pages` to get both text and page count for the quality heuristic, then join pages.

## Sources

### Primary (HIGH confidence)
- [pulldown-cmark 0.13.3 docs](https://docs.rs/pulldown-cmark/latest) - Event, Tag, TagEnd enums, Parser API
- [pdf-extract 0.10.0 docs](https://docs.rs/pdf-extract/latest) - extract_text, extract_text_by_pages API
- [indicatif docs](https://docs.rs/indicatif/latest) - ProgressBar::new(total), ProgressStyle templates
- Existing codebase: `src/commands/generate.rs`, `src/audio/mod.rs`, `src/bridge/inference.rs`, `src/ui.rs`

### Secondary (MEDIUM confidence)
- [pulldown-cmark GitHub](https://github.com/pulldown-cmark/pulldown-cmark) - Event iterator patterns
- [crates.io version verification](https://crates.io) - pulldown-cmark 0.13.3, pdf-extract 0.10.0 confirmed current

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH - Libraries locked in CLAUDE.md, versions verified on crates.io
- Architecture: HIGH - Clear integration points in existing generate.rs, well-understood patterns
- Pitfalls: MEDIUM - TTS input length limits are empirical, PDF quality heuristics need tuning in practice

**Research date:** 2026-03-28
**Valid until:** 2026-04-28 (stable libraries, no fast-moving components)
