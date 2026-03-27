# Roadmap: Chatter

## Overview

Chatter delivers a Rust CLI for local text-to-speech with voice profile management, built on Qwen3-TTS via PyO3. The roadmap moves from establishing the Python bridge (the binary blocker) through voice profiles and core speech generation (the product differentiator) to file input processing (the convenience layer). Each phase delivers a complete, testable capability.

## Phases

**Phase Numbering:**
- Integer phases (1, 2, 3): Planned milestone work
- Decimal phases (2.1, 2.2): Urgent insertions (marked with INSERTED)

Decimal phases appear between their surrounding integers in numeric order.

- [ ] **Phase 1: Foundation and Python Bridge** - Working Rust binary that initializes Python, loads Qwen3-TTS, and validates the environment
- [ ] **Phase 2: Voice Profiles and Speech Generation** - Users can design/clone voice profiles and generate speech from text
- [ ] **Phase 3: File Input and Text Processing** - Users can generate speech from TXT, Markdown, and PDF files

## Phase Details

### Phase 1: Foundation and Python Bridge
**Goal**: Users can run the chatter binary, see helpful CLI usage, and confirm their environment (Python, GPU, qwen-tts) is ready for TTS work
**Depends on**: Nothing (first phase)
**Requirements**: FOUN-01, FOUN-02, FOUN-03, FOUN-04, FOUN-05, UX-01, UX-04
**Success Criteria** (what must be TRUE):
  1. User can run `chatter --help` and see usage for design, clone, generate, and profiles subcommands
  2. User can run `chatter` and it initializes the Python runtime and loads a Qwen3-TTS model with a visible progress bar
  3. User sees a clear, actionable error message when GPU is unavailable or Python/qwen-tts is not installed
  4. User can pass `--language` and `--model-size` flags that are validated against allowed values
**Plans:** 3 plans

Plans:
- [x] 01-01-PLAN.md -- Rust project scaffold and CLI argument parsing
- [x] 01-02-PLAN.md -- PyO3 bridge and model management commands
- [x] 01-03-PLAN.md -- Doctor command and shared UI helpers

### Phase 2: Voice Profiles and Speech Generation
**Goal**: Users can create reusable voice profiles (by description or cloning) and generate speech from inline text using those profiles
**Depends on**: Phase 1
**Requirements**: PROF-01, PROF-02, PROF-03, PROF-04, PROF-05, PROF-06, GEN-01, GEN-05, GEN-06, UX-02, UX-03
**Success Criteria** (what must be TRUE):
  1. User can run `chatter design` with a natural language description and get a saved voice profile with cached sample audio
  2. User can run `chatter clone` with a reference MP3 and get a saved voice profile with cached sample audio
  3. User can run `chatter profiles list` and see all saved profiles with name, type, language, and creation date
  4. User can run `chatter generate "some text" --profile myvoice` and get an MP3 file of spoken audio
  5. User sees progress bars during voice profile creation and speech synthesis
**Plans:** 4 plans

Plans:
- [x] 02-01-PLAN.md -- Core types, profile storage, audio pipeline, CLI cleanup, Python bridge adapter
- [ ] 02-02-PLAN.md -- Voice design command with interactive preview loop
- [ ] 02-03-PLAN.md -- Voice clone command and profiles list/show
- [ ] 02-04-PLAN.md -- Speech generation command

### Phase 3: File Input and Text Processing
**Goal**: Users can generate speech from document files (TXT, Markdown, PDF) using saved voice profiles
**Depends on**: Phase 2
**Requirements**: GEN-02, GEN-03, GEN-04
**Success Criteria** (what must be TRUE):
  1. User can run `chatter generate --file document.txt --profile myvoice` and get spoken audio output
  2. User can run `chatter generate --file notes.md --profile myvoice` and Markdown formatting is stripped before synthesis
  3. User can run `chatter generate --file paper.pdf --profile myvoice` and text is extracted and synthesized
**Plans**: TBD

## Progress

**Execution Order:**
Phases execute in numeric order: 1 -> 2 -> 3

| Phase | Plans Complete | Status | Completed |
|-------|----------------|--------|-----------|
| 1. Foundation and Python Bridge | 3/3 | Complete |  |
| 2. Voice Profiles and Speech Generation | 0/4 | Not started | - |
| 3. File Input and Text Processing | 0/0 | Not started | - |
