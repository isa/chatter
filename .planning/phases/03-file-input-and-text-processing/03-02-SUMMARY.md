---
phase: 03-file-input-and-text-processing
plan: 02
subsystem: generate-command
tags: [file-input, multi-chunk, synthesis, progress-bar, split-output, pcm-concatenation]

# Dependency graph
requires:
  - "extract::extract_text from Plan 03-01"
  - "chunk::chunk_by_paragraph from Plan 03-01"
provides:
  - "File-to-speech pipeline in generate command (TXT, Markdown, PDF)"
  - "Multi-chunk synthesis with bounded progress bar"
  - "Split output mode producing numbered per-chunk MP3 files"
  - "--split CLI flag on generate command"
  - "create_progress_bar() bounded progress bar helper"
affects:
  - "src/commands/generate.rs"
  - "src/cli.rs"
  - "src/ui.rs"

# Tech stack
added: []
patterns:
  - "PCM concatenation with configurable silence gaps between chunks"
  - "Bounded progress bar for multi-item synthesis loops"
  - "Split output path generation with 3-digit zero-padding"

# Key files
created: []
modified:
  - "src/commands/generate.rs"
  - "src/cli.rs"
  - "src/ui.rs"

# Decisions
key-decisions:
  - "300ms silence gap between concatenated chunks (research recommendation)"
  - "3-digit zero-padded numbering for split output files"
  - "Skip audio playback in split mode (no single file to play)"

# Metrics
duration: "2min"
completed: "2026-03-28"
---

# Phase 03 Plan 02: File Input Generate Command Wiring Summary

Multi-chunk file-to-speech pipeline with bounded progress bar, PCM concatenation with 300ms silence gaps, and split output mode producing numbered MP3 files.

## What Was Done

### Task 1: Add --split flag and bounded progress bar helper
- Added `pub split: bool` with `#[arg(long)]` to `GenerateArgs` in `src/cli.rs`
- Added `create_progress_bar(total, message)` to `src/ui.rs` with `{pos}/{len}` bounded display and `{bar:30}` visual bar
- **Commit:** 399c2f2

### Task 2: Wire file extraction, chunking, multi-chunk synthesis, and split output
- Replaced the "file input not yet supported" bail with actual `extract::extract_text()` call wrapped in a "Reading file..." spinner
- Added three helper functions: `silence_samples()`, `concatenate_chunks()`, `split_output_path()`
- Text is chunked via `chunk::chunk_by_paragraph()` before synthesis
- Single chunk uses spinner (backward compatible with inline text); 2+ chunks use bounded progress bar
- Default mode concatenates all PCM chunks with 300ms silence gaps into one MP3
- Split mode (`--split`) encodes each chunk as a separate numbered MP3 (e.g., `output-001.mp3`)
- Playback skipped in split mode since there is no single output file
- **Commit:** aad2940

## Deviations from Plan

None -- plan executed exactly as written.

## Verification Results

- `cargo check` compiles successfully with no errors
- `src/commands/generate.rs` contains all required imports (`use crate::chunk`, `use crate::extract`)
- Old "File input is not yet supported" bail message removed
- File input path: extract -> chunk -> synthesize loop -> concatenate -> encode
- Split path: extract -> chunk -> synthesize loop -> encode per chunk
- Progress: spinner for read/extract/encode steps, bounded bar for multi-chunk synthesis

## Known Stubs

None -- all functionality is fully wired end-to-end.

## Self-Check: PASSED
