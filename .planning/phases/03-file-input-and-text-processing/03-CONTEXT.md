# Phase 3: File Input and Text Processing - Context

**Gathered:** 2026-03-28
**Status:** Ready for planning

<domain>
## Phase Boundary

Users can generate speech from document files (TXT, Markdown, PDF) via `chatter generate --file <path>`. Text is extracted, cleaned, chunked by paragraph, synthesized per-chunk, and concatenated into a single MP3 output (or split files with `--split`). This phase adds file reading and text processing to the existing generate command — no new subcommands.

</domain>

<decisions>
## Implementation Decisions

### File Type Detection
- **D-01:** Extension-based detection with content validation. Check file extension first (.txt, .md/.markdown, .pdf), then validate content (e.g., PDF magic bytes `%PDF`). Catches misnamed files.
- **D-02:** Unsupported file types (.docx, .html, etc.) are attempted as plain UTF-8 text with a warning. If the file contains binary/non-UTF-8 content, fail gracefully with a clear error.

### Long Document Handling
- **D-03:** Chunk text by paragraph breaks (double newline). Each paragraph becomes a separate synthesis call. Audio from all chunks is concatenated into one MP3.
- **D-04:** Single output file by default. Add `--split` flag to produce numbered separate files per chunk (e.g., `output-001.mp3`, `output-002.mp3`, ...).

### Text Extraction — Markdown
- **D-05:** Strip Markdown markup but preserve structure hints. Add pauses (extra newlines / silence) after headers and between sections so the audio flows naturally for structured documents.
- **D-06:** Code blocks are replaced with a spoken placeholder: "A code block appears here. See the original document for details." — with pauses before and after. Code content is not read aloud.
- **D-07:** Images and diagrams in Markdown are replaced with a spoken placeholder: "An image appears here. See the original document for details." — same pause treatment as code blocks.

### Text Extraction — PDF
- **D-08:** Best-effort extraction using `pdf-extract` crate. If extracted text looks short relative to page count or contains garbled characters (heuristic), warn the user but proceed with what was extracted.
- **D-09:** Images and diagrams in PDFs get the same spoken placeholder treatment as Markdown (D-07).

### Text Extraction — TXT
- **D-10:** Read as-is. UTF-8 encoding assumed. No processing beyond paragraph chunking.

### Progress Feedback
- **D-11:** Multi-step progress display with distinct phases: "Reading file..." -> "Extracting text..." -> "Synthesizing (3/12)..." -> "Encoding MP3..."
- **D-12:** Synthesis step uses a bounded progress bar showing chunk progress: `[========>           ] 3/12 chunks`. Other steps (reading, extracting, encoding) use spinners consistent with Phase 1/2 style.

### Claude's Discretion
- Exact heuristics for PDF quality detection (garbled text, short extraction)
- Paragraph chunking edge cases (very long paragraphs, single-line documents)
- How silence/pauses are inserted between chunks and after headers (duration, method)
- `--split` flag naming convention for output files
- How `pulldown-cmark` events map to text extraction (which AST nodes to strip vs preserve)
- MP3 concatenation approach (concatenate PCM samples before encoding, or encode per-chunk and join)

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Project Definition
- `.planning/PROJECT.md` — Core value, constraints, key decisions
- `.planning/REQUIREMENTS.md` — GEN-02, GEN-03, GEN-04 are Phase 3 requirements
- `.planning/ROADMAP.md` — Phase 3 success criteria and dependency chain

### Technology Stack
- `CLAUDE.md` §Technology Stack — `pdf-extract 0.10.0` for PDF, `pulldown-cmark 0.13.3` for Markdown, `hound` for WAV, `mp3lame-encoder` for MP3

### Prior Phase Context
- `.planning/phases/01-foundation-and-python-bridge/01-CONTEXT.md` — Progress feedback patterns (D-07/D-08), error presentation (D-05/D-06)
- `.planning/phases/02-voice-profiles-and-speech-generation/02-CONTEXT.md` — Generate command behavior (D-15 through D-18), audio pipeline patterns

### Existing Code
- `src/commands/generate.rs` — Current generate command with `--file` placeholder (line 41-53)
- `src/cli.rs` — `GenerateArgs` already has `file: Option<PathBuf>` field
- `src/audio/mod.rs` — WAV-to-MP3 encoding pipeline (reuse for chunk concatenation)
- `src/ui.rs` — `create_spinner()` for progress feedback

### External References
- [Qwen3-TTS GitHub](https://github.com/QwenLM/Qwen3-TTS) — Model input length limits and behavior
- [pulldown-cmark docs](https://docs.rs/pulldown-cmark/latest) — Markdown parsing API
- [pdf-extract docs](https://docs.rs/pdf-extract/latest) — PDF text extraction API

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `src/commands/generate.rs`: Full generate pipeline (profile loading, inference, MP3 encoding, playback). File input wires into the text extraction step before the existing synthesis flow.
- `src/audio/mod.rs`: `samples_f32_to_i16()` and `encode_wav_to_mp3()` — reuse for per-chunk encoding or final concatenated encoding.
- `src/ui.rs`: `create_spinner()` — reuse for read/extract/encode steps. Need to add bounded progress bar for synthesis step.
- `src/bridge/inference.rs`: `generate_speech()` — called per chunk, returns `(Vec<f32>, u32)`.

### Established Patterns
- PyO3 `Python::attach(|py| { ... })` for all Python interop
- `BridgeError` enum with `thiserror` for typed errors
- `indicatif` spinners with `finish_and_clear()` pattern
- owo-colors with `if_supports_color` for NO_COLOR compliance

### Integration Points
- `src/commands/generate.rs` lines 41-53: Replace the `--file` bail with actual file reading + text extraction
- `Cargo.toml`: Add `pdf-extract` and `pulldown-cmark` dependencies
- `src/cli.rs`: Add `--split` flag to `GenerateArgs`

</code_context>

<specifics>
## Specific Ideas

- Code blocks, images, and diagrams should never be read aloud. Replace with spoken placeholders that orient the listener: "A code block appears here. See the original document for details." This applies to both Markdown and PDF content.
- Structure-aware Markdown stripping: headers get pauses after them, not just text extraction. The goal is natural-sounding audio for structured documents.
- PDF extraction is best-effort with quality warnings — don't block the user, but let them know if results might be poor.

</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope.

</deferred>

---

*Phase: 03-file-input-and-text-processing*
*Context gathered: 2026-03-28*
