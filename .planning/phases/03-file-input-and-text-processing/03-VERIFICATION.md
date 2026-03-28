---
phase: 03-file-input-and-text-processing
verified: 2026-03-28T18:00:00Z
status: passed
score: 13/13 must-haves verified
re_verification: false
---

# Phase 03: File Input and Text Processing Verification Report

**Phase Goal:** Users can generate speech from document files (TXT, Markdown, PDF) using saved voice profiles
**Verified:** 2026-03-28T18:00:00Z
**Status:** passed
**Re-verification:** No — initial verification

## Goal Achievement

### Observable Truths

| #  | Truth                                                                                                           | Status     | Evidence                                                                     |
|----|-----------------------------------------------------------------------------------------------------------------|------------|------------------------------------------------------------------------------|
| 1  | TXT files are read as UTF-8 and returned as-is                                                                  | VERIFIED   | `src/extract/txt.rs`: `fs::read_to_string(path).context(...)` — no transform |
| 2  | Markdown files have markup stripped, code blocks and images replaced with spoken placeholders, headers produce paragraph breaks | VERIFIED   | `src/extract/markdown.rs` lines 29-33, 38-40, 21-27 implement all three     |
| 3  | PDF files have text extracted with quality heuristics warning on poor extraction; pages with very little text are flagged as possibly image-heavy | VERIFIED   | `src/extract/pdf.rs` lines 21-34 (quality heuristics), lines 36-74 (per-page heuristic via form-feed splitting) |
| 4  | Unknown file extensions are attempted as plain UTF-8 with a warning                                            | VERIFIED   | `src/extract/mod.rs` lines 42-56: eprintln warning then `txt::extract` fallback with binary-content error |
| 5  | Text is split into chunks by double newline with empty chunks filtered                                          | VERIFIED   | `src/chunk.rs` line 9: `.split("\n\n")`, line 11: `.filter(|s| !s.is_empty())` |
| 6  | Very long paragraphs (>3000 chars) are sub-split at sentence boundaries                                         | VERIFIED   | `src/chunk.rs` lines 13-18, `split_long_paragraph` function (lines 27-73)   |
| 7  | User can run `chatter generate --file document.txt --profile myvoice` and get spoken audio output              | VERIFIED   | `src/commands/generate.rs` lines 96-106: file branch with extract wiring; lines 167-193: synthesis loop |
| 8  | User can run `chatter generate --file notes.md --profile myvoice` and Markdown formatting is stripped          | VERIFIED   | Markdown dispatcher in `extract/mod.rs` line 40 calls `markdown::extract`; full markup stripping confirmed |
| 9  | User can run `chatter generate --file paper.pdf --profile myvoice` and text is extracted and synthesized       | VERIFIED   | PDF dispatcher in `extract/mod.rs` line 41 calls `pdf::extract`; synthesized via chunk loop |
| 10 | Multi-chunk files produce a single concatenated MP3 by default                                                  | VERIFIED   | `generate.rs` lines 219-237: `concatenate_chunks` with 300ms gaps, single `encode_wav_to_mp3` call |
| 11 | User can pass --split to get numbered per-chunk MP3 files                                                       | VERIFIED   | `cli.rs` line 118: `pub split: bool`; `generate.rs` lines 196-218: split branch with `split_output_path` |
| 12 | Progress bar shows chunk progress during synthesis (3/12 format)                                                | VERIFIED   | `ui.rs` lines 23-35: `create_progress_bar` with `{pos}/{len}` and `{bar:30}` template; called at `generate.rs` line 184 |
| 13 | Spinners show during file reading and text extraction steps                                                      | VERIFIED   | `generate.rs` line 98: `create_spinner("Reading file...")`; lines 176, 198, 221: spinners for synthesis and encoding |

**Score:** 13/13 truths verified

### Required Artifacts

| Artifact                      | Expected                                                                  | Status   | Details                                                                     |
|-------------------------------|---------------------------------------------------------------------------|----------|-----------------------------------------------------------------------------|
| `src/extract/mod.rs`          | FileFormat enum, detect_format(), extract_text() dispatcher               | VERIFIED | Exports `FileFormat`, `detect_format`, `extract_text`; all dispatch arms present |
| `src/extract/txt.rs`          | TXT extraction (UTF-8 read)                                               | VERIFIED | `pub fn extract(path: &Path) -> anyhow::Result<String>` — 8 lines, substantive |
| `src/extract/markdown.rs`     | Markdown extraction via pulldown-cmark                                    | VERIFIED | Full event loop, code/image placeholders, heading breaks, table skipping    |
| `src/extract/pdf.rs`          | PDF extraction via pdf-extract with quality heuristics and image-page detection | VERIFIED | Magic byte check, quality heuristics, form-feed page splitting, placeholder insertion |
| `src/chunk.rs`                | Paragraph chunking with long-paragraph sub-splitting                      | VERIFIED | `chunk_by_paragraph`, `MAX_CHUNK_CHARS = 3000`, `split_long_paragraph`, `split_at_spaces` |
| `src/commands/generate.rs`    | File input branch with extraction, chunking, multi-chunk synthesis, PCM concatenation, split output | VERIFIED | All branches present; `extract::extract_text` and `chunk::chunk_by_paragraph` called |
| `src/cli.rs`                  | --split flag on GenerateArgs                                              | VERIFIED | `pub split: bool` with `#[arg(long)]` at line 117-118                      |
| `src/ui.rs`                   | create_progress_bar() for bounded chunk progress                          | VERIFIED | `pub fn create_progress_bar(total: u64, message: &str) -> ProgressBar` at line 23 |

### Key Link Verification

| From                           | To                             | Via                                      | Status   | Details                                                          |
|--------------------------------|--------------------------------|------------------------------------------|----------|------------------------------------------------------------------|
| `src/extract/mod.rs`           | `src/extract/txt.rs`           | `txt::extract(path)`                     | WIRED    | Line 39: `FileFormat::Txt => txt::extract(path)`                |
| `src/extract/mod.rs`           | `src/extract/markdown.rs`      | `markdown::extract(path)`                | WIRED    | Line 40: `FileFormat::Markdown => markdown::extract(path)`      |
| `src/extract/mod.rs`           | `src/extract/pdf.rs`           | `pdf::extract(path)`                     | WIRED    | Line 41: `FileFormat::Pdf => pdf::extract(path)`                |
| `src/commands/generate.rs`     | `src/extract/mod.rs`           | `extract::extract_text(file_path)`       | WIRED    | Line 99: `extract::extract_text(file_path)`; `use crate::extract` line 11 |
| `src/commands/generate.rs`     | `src/chunk.rs`                 | `chunk::chunk_by_paragraph(&text)`       | WIRED    | Line 167: `chunk::chunk_by_paragraph(&text)`; `use crate::chunk` line 9 |
| `src/commands/generate.rs`     | `src/bridge/inference.rs`      | `inference::generate_speech` per chunk  | WIRED    | Lines 177, 186: `inference::generate_speech` called in both synthesis branches |
| `src/commands/generate.rs`     | `src/audio/mod.rs`             | `audio::encode_wav_to_mp3` for final output | WIRED | Lines 202, 228: `audio::encode_wav_to_mp3` in both split and concat paths |

### Data-Flow Trace (Level 4)

| Artifact                    | Data Variable | Source                                   | Produces Real Data              | Status    |
|-----------------------------|---------------|------------------------------------------|---------------------------------|-----------|
| `src/commands/generate.rs`  | `text`        | `extract::extract_text(file_path)`       | Reads file from disk at runtime | FLOWING   |
| `src/commands/generate.rs`  | `chunks`      | `chunk::chunk_by_paragraph(&text)`       | Derived from `text` above       | FLOWING   |
| `src/commands/generate.rs`  | `audio_parts` | `inference::generate_speech` per chunk  | PyO3 call to Python TTS model   | FLOWING   |
| `src/commands/generate.rs`  | MP3 output    | `audio::encode_wav_to_mp3`               | Writes to disk from audio_parts | FLOWING   |

### Behavioral Spot-Checks

| Behavior                              | Command                                                                                                     | Result        | Status |
|---------------------------------------|-------------------------------------------------------------------------------------------------------------|---------------|--------|
| cargo check passes                    | `cargo check 2>&1 \| tail -3`                                                                               | `Finished dev profile` — 15 warnings, 0 errors | PASS   |
| mod extract declared in main.rs       | grep in src/main.rs                                                                                          | Line 6: `mod extract;` | PASS  |
| mod chunk declared in main.rs         | grep in src/main.rs                                                                                          | Line 3: `mod chunk;`   | PASS  |
| Old stub bail removed from generate   | grep for "File input is not yet supported"                                                                   | No matches found        | PASS  |
| --split flag present in GenerateArgs  | Read src/cli.rs lines 116-118                                                                               | `pub split: bool` confirmed | PASS |

### Requirements Coverage

| Requirement | Source Plans     | Description                                                                      | Status    | Evidence                                                              |
|-------------|-----------------|----------------------------------------------------------------------------------|-----------|-----------------------------------------------------------------------|
| GEN-02      | 03-01, 03-02    | User can generate speech from a TXT file path via `chatter generate --file <path>` | SATISFIED | `txt::extract` reads UTF-8, wired through `extract_text` dispatcher, called from `generate.rs` file branch |
| GEN-03      | 03-01, 03-02    | User can generate speech from a Markdown file path (formatting stripped before synthesis) | SATISFIED | `markdown::extract` uses pulldown-cmark event loop to strip markup and replace code/image blocks with spoken text |
| GEN-04      | 03-01, 03-02    | User can generate speech from a PDF file path (basic text extraction)           | SATISFIED | `pdf::extract` uses pdf-extract library with quality heuristics and image-page detection |

No orphaned requirements found — all three requirement IDs (GEN-02, GEN-03, GEN-04) appear in both plan frontmatter fields and are confirmed in REQUIREMENTS.md.

### Anti-Patterns Found

No blockers or stubs detected.

| File                              | Pattern Checked                    | Result   | Notes                                              |
|-----------------------------------|------------------------------------|----------|----------------------------------------------------|
| `src/extract/mod.rs`              | return null / placeholder text     | Clean    | Functional dispatcher with all arms implemented    |
| `src/extract/txt.rs`              | empty implementation               | Clean    | Single real read_to_string call                    |
| `src/extract/markdown.rs`         | TODO/FIXME/placeholder             | Clean    | Full pulldown-cmark event loop                     |
| `src/extract/pdf.rs`              | empty returns / static data        | Clean    | Real pdf_extract_text call with heuristics         |
| `src/chunk.rs`                    | return []                          | Clean    | Real split logic with sub-splitting                |
| `src/commands/generate.rs`        | "not yet supported" / bail stub    | Clean    | Old stub removed; full pipeline wired              |

### Human Verification Required

The following items cannot be verified programmatically and require a working GPU environment with the Qwen3-TTS model installed:

#### 1. End-to-End TXT File Speech Generation

**Test:** Run `chatter generate --file /path/to/hello.txt --profile myvoice`
**Expected:** A playable MP3 file is produced containing spoken audio of the TXT file's contents
**Why human:** Requires Python venv with qwen-tts installed and a GPU; cannot be tested statically

#### 2. Markdown Formatting Strip — Audible Quality

**Test:** Run `chatter generate --file /path/to/README.md --profile myvoice` on a markdown file with headers, code blocks, and images
**Expected:** Code blocks and images produce spoken placeholder sentences; headers cause natural pauses; no raw markdown syntax is spoken aloud
**Why human:** Audible output quality requires a human listener

#### 3. PDF Image-Page Placeholder Insertion

**Test:** Run `chatter generate --file /path/to/image-heavy.pdf --profile myvoice` on a PDF with image-only pages
**Expected:** Warning printed about image pages; placeholder text spoken in place of image-only pages
**Why human:** Requires a real multi-page PDF with image-only pages and audio playback to confirm

#### 4. Split Mode Output Naming

**Test:** Run `chatter generate --file /path/to/long.txt --profile myvoice --output out.mp3 --split` on a file that produces 3+ chunks
**Expected:** Files `out-001.mp3`, `out-002.mp3`, etc. created; single-file `out.mp3` not created
**Why human:** Requires model and full synthesis pipeline to execute

#### 5. Multi-Chunk Silence Gap Audibility

**Test:** Generate from a multi-paragraph file without `--split`
**Expected:** Natural 300ms pauses audible between paragraphs in the concatenated MP3
**Why human:** Silence gap quality is subjective and requires audio playback

---

## Summary

Phase 03 goal is fully achieved. All 13 observable truths are verified with substantive implementations, complete wiring, and real data flow. The three requirement IDs (GEN-02, GEN-03, GEN-04) are each satisfied by concrete code in the codebase, not stubs. The generate command's old "file input is not yet supported" bail has been replaced with a real extraction-chunking-synthesis pipeline. Cargo check passes with zero errors and 15 pre-existing warnings. Five items require human verification with a live model environment.

---

_Verified: 2026-03-28T18:00:00Z_
_Verifier: Claude (gsd-verifier)_
