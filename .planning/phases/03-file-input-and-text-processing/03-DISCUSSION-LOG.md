# Phase 3: File Input and Text Processing - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-03-28
**Phase:** 03-file-input-and-text-processing
**Areas discussed:** File type detection, Long document handling, Text extraction quality, Progress feedback

---

## File Type Detection

| Option | Description | Selected |
|--------|-------------|----------|
| Extension-based | .txt, .md/.markdown, .pdf. Simple, predictable, no magic. Error if unrecognized extension. | |
| Content sniffing + extension | Check extension first, but also validate (e.g., PDF magic bytes). Catches misnamed files. | ✓ |
| Explicit --format flag | User specifies format. Extension as fallback if flag omitted. | |

**User's choice:** Content sniffing + extension
**Notes:** None

### Follow-up: Unsupported file types

| Option | Description | Selected |
|--------|-------------|----------|
| Hard error | "Unsupported file type. Supported: .txt, .md, .pdf" | |
| Try as plain text | Attempt to read as UTF-8 text with a warning. Fails gracefully if binary. | ✓ |

**User's choice:** Try as plain text
**Notes:** Graceful fallback for unknown extensions

---

## Long Document Handling

| Option | Description | Selected |
|--------|-------------|----------|
| Single synthesis call | Send entire text to qwen-tts in one shot. Simplest but may hit limits. | |
| Chunk by paragraphs | Split on paragraph breaks, synthesize each, concatenate audio. | ✓ |
| Hard length limit | Truncate at max char count with warning. | |

**User's choice:** Chunk by paragraphs
**Notes:** None

### Follow-up: Chunk splitting method

| Option | Description | Selected |
|--------|-------------|----------|
| Double newline (paragraph breaks) | Split on blank lines. Natural, preserves sentence flow. | ✓ |
| Sentence-level | Split on sentence boundaries. More granular. | |
| You decide | Claude picks best strategy. | |

**User's choice:** Double newline (paragraph breaks)
**Notes:** None

### Follow-up: Output files

| Option | Description | Selected |
|--------|-------------|----------|
| Single output file | Concatenate all chunks into one MP3. | |
| Separate files per chunk | Output numbered files (doc-001.mp3, etc.). | |
| Single by default, --split flag for separate | Best of both, adds a flag. | ✓ |

**User's choice:** Single by default, --split flag for separate
**Notes:** None

---

## Text Extraction Quality

### Markdown formatting

| Option | Description | Selected |
|--------|-------------|----------|
| Strip all markup | Remove headers, bold, links, images, code fences. Just plain text. | |
| Preserve structure hints | Strip markup but add pauses after headers. More natural audio flow. | ✓ |
| You decide | Claude picks best approach. | |

**User's choice:** Preserve structure hints
**Notes:** None

### Code blocks in Markdown

| Option | Description | Selected |
|--------|-------------|----------|
| Skip entirely | Code blocks removed from text. | |
| Read as-is | Include code content as plain text. | |
| Skip by default, --include-code to read | Sensible default with escape hatch. | |

**User's choice:** (Other) Replace code blocks with a spoken placeholder like "A code block appears here. See the original document for details." with pauses before and after.
**Notes:** User preferred an orientation cue over silence or reading code aloud.

### PDF extraction quality

| Option | Description | Selected |
|--------|-------------|----------|
| Best effort, no warnings | Extract what we can, skip what fails. | |
| Best effort with quality warning | If text looks short or garbled, warn but proceed. | ✓ |
| Strict validation | Error out if quality is poor. | |

**User's choice:** Best effort with quality warning
**Notes:** None

---

## Progress Feedback

### Overall progress model

| Option | Description | Selected |
|--------|-------------|----------|
| Per-chunk progress bar | "Synthesizing chunk 3/12..." with bounded bar. | |
| Single spinner with chunk count | Spinner updates: "Generating speech... (chunk 3 of 12)". | |
| Multi-step display | Distinct phases: Reading -> Extracting -> Synthesizing -> Encoding. | ✓ |

**User's choice:** Multi-step display
**Notes:** None

### Synthesis step specifically

| Option | Description | Selected |
|--------|-------------|----------|
| Progress bar | [========>           ] 3/12 chunks — visual and bounded. | ✓ |
| Spinner with count | "Synthesizing (3/12)..." — lighter, consistent with earlier phases. | |

**User's choice:** Progress bar
**Notes:** None

### Additional user input

**User added:** Images and diagrams should also get spoken placeholder treatment, same as code blocks: "An image appears here. See the original document for details." with pauses.

---

## Claude's Discretion

- Exact heuristics for PDF quality detection
- Paragraph chunking edge cases
- Silence/pause duration and insertion method
- `--split` output file naming convention
- pulldown-cmark event-to-text mapping details
- MP3 concatenation approach (PCM-level vs file-level)

## Deferred Ideas

None — discussion stayed within phase scope.
