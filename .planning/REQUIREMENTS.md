# Requirements: Chatter

**Defined:** 2026-03-27
**Core Value:** Users can create reusable voice profiles and generate high-quality speech from text or documents without leaving the command line.

## v1 Requirements

Requirements for initial release. Each maps to roadmap phases.

### Foundation

- [x] **FOUN-01**: PyO3 bridge initializes Python runtime and loads qwen-tts models successfully
- [x] **FOUN-02**: CLI binary parses subcommands (design, clone, generate) with appropriate flags
- [x] **FOUN-03**: Language flag accepts: Auto, Chinese, English, Japanese, Korean, French, German, Spanish, Portuguese, Russian, Italian
- [x] **FOUN-04**: Model size flag accepts 0.6B and 1.7B (default 1.7B)
- [x] **FOUN-05**: Helpful error messages when GPU unavailable or Python/qwen-tts not installed

### Voice Profiles

- [ ] **PROF-01**: User can create a voice profile from a natural language description via `chatter design`
- [ ] **PROF-02**: User can create a voice profile from a reference MP3 file via `chatter clone`
- [x] **PROF-03**: Voice profiles are saved to `~/.config/chatter/profiles/` with metadata (TOML) and cached sample audio (MP3)
- [ ] **PROF-04**: User can list all saved voice profiles via `chatter profiles list`
- [x] **PROF-05**: Profile metadata includes: name, type (designed/cloned), language, description/source, creation date
- [ ] **PROF-06**: Cached sample audio is generated at profile creation time for previewing

### Speech Generation

- [ ] **GEN-01**: User can generate speech from inline text using a saved voice profile via `chatter generate`
- [ ] **GEN-02**: User can generate speech from a TXT file path via `chatter generate --file <path>`
- [ ] **GEN-03**: User can generate speech from a Markdown file path (formatting stripped before synthesis)
- [ ] **GEN-04**: User can generate speech from a PDF file path (basic text extraction)
- [ ] **GEN-05**: Generated audio is saved as MP3 to a user-specified or default output path
- [ ] **GEN-06**: Language flag on generate overrides the profile's default language when specified

### User Experience

- [x] **UX-01**: Progress bar displays during model loading
- [ ] **UX-02**: Progress bar displays during speech synthesis
- [ ] **UX-03**: Progress bar displays during voice profile creation (design and clone)
- [x] **UX-04**: `--help` provides clear usage information for all commands and flags

## v2 Requirements

Deferred to future release. Tracked but not in current roadmap.

### Enhanced Input

- **INP-01**: User can pipe text via stdin (`echo "hello" | chatter generate`)
- **INP-02**: Long document chunking with per-chunk progress for large files
- **INP-03**: Raw audio stdout piping (`--pipe-out`) for Unix pipeline integration

### Profile Management

- **PMGT-01**: User can delete a voice profile via `chatter profiles delete`
- **PMGT-02**: User can preview/play a profile's cached sample via `chatter profiles preview`
- **PMGT-03**: User can export/import voice profiles for sharing

### Audio

- **AUD-01**: Additional output formats (OGG, FLAC, WAV)
- **AUD-02**: Shell completions for bash, zsh, fish

## Out of Scope

Explicitly excluded. Documented to prevent scope creep.

| Feature | Reason |
|---------|--------|
| Cloud/DashScope API | Contradicts local-first value; requires API keys and billing |
| Streaming audio playback | Platform-specific audio device complexity; users can play generated files |
| GUI or TUI | Different product entirely; CLI-first |
| SSML markup | Qwen3-TTS doesn't support SSML; natural language descriptions handle style |
| Batch file processing | Easy to script with shell loops; single file per invocation keeps it simple |
| Voice blending/mixing | Not natively supported by Qwen3-TTS; high complexity |
| Web server / API mode | Different product category; stay CLI |
| Real-time voice conversion | Different use case entirely |

## Traceability

Which phases cover which requirements. Updated during roadmap creation.

| Requirement | Phase | Status |
|-------------|-------|--------|
| FOUN-01 | Phase 1 | Complete |
| FOUN-02 | Phase 1 | Complete |
| FOUN-03 | Phase 1 | Complete |
| FOUN-04 | Phase 1 | Complete |
| FOUN-05 | Phase 1 | Complete |
| PROF-01 | Phase 2 | Pending |
| PROF-02 | Phase 2 | Pending |
| PROF-03 | Phase 2 | Complete |
| PROF-04 | Phase 2 | Pending |
| PROF-05 | Phase 2 | Complete |
| PROF-06 | Phase 2 | Pending |
| GEN-01 | Phase 2 | Pending |
| GEN-02 | Phase 3 | Pending |
| GEN-03 | Phase 3 | Pending |
| GEN-04 | Phase 3 | Pending |
| GEN-05 | Phase 2 | Pending |
| GEN-06 | Phase 2 | Pending |
| UX-01 | Phase 1 | Complete |
| UX-02 | Phase 2 | Pending |
| UX-03 | Phase 2 | Pending |
| UX-04 | Phase 1 | Complete |

**Coverage:**
- v1 requirements: 21 total
- Mapped to phases: 21
- Unmapped: 0

---
*Requirements defined: 2026-03-27*
*Last updated: 2026-03-27 after roadmap creation*
