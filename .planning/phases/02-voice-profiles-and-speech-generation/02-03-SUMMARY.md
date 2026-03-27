---
phase: 02-voice-profiles-and-speech-generation
plan: 03
subsystem: cli
tags: [voice-cloning, profile-management, audio-validation, hound, owo-colors]

# Dependency graph
requires:
  - phase: 02-01
    provides: "Profile storage, bridge inference, audio encoding, CLI structure"
provides:
  - "Clone command: voice cloning from reference audio with input validation"
  - "Profiles list command: formatted table of saved profiles"
  - "Profiles show command: full profile detail with file sizes"
  - "Profiles delete command: profile removal with confirmation"
affects: [02-04]

# Tech tracking
tech-stack:
  added: []
  patterns: [language-enum-mapping, wav-validation-with-hound, dynamic-column-width-tables]

key-files:
  created: []
  modified:
    - src/commands/clone.rs
    - src/commands/profiles.rs

key-decisions:
  - "Used hound WavReader for WAV validation (duration, sample rate) but skip MP3 validation beyond size check"
  - "Clone generates preview sample before saving prompt (needs ref audio for both)"
  - "Profiles delete uses stdin confirmation rather than CLI prompt crate"

patterns-established:
  - "language_to_string() maps Language enum to Python bridge strings"
  - "format_size() for human-readable byte sizes in profile show output"
  - "validate_audio_file() pattern: existence -> extension -> size -> format-specific checks"

requirements-completed: [PROF-02, PROF-04]

# Metrics
duration: 2min
completed: 2026-03-27
---

# Phase 02 Plan 03: Clone and Profiles Summary

**Voice cloning command with WAV/MP3 validation and profile management (list/show/delete) with formatted table output**

## Performance

- **Duration:** 2 min
- **Started:** 2026-03-27T18:25:32Z
- **Completed:** 2026-03-27T18:27:31Z
- **Tasks:** 2
- **Files modified:** 2

## Accomplishments
- Clone command validates input (existence, format, WAV duration/sample rate), clones voice via bridge, saves profile with sample.mp3 and voice_prompt.bin
- Profiles list displays formatted table with dynamic column widths, dimmed separator, and date-only created column
- Profiles show displays full metadata, source audio path, model variant, and file sizes
- Profiles delete with confirmation prompt and --yes skip flag

## Task Commits

Each task was committed atomically:

1. **Task 1: Implement chatter clone command with input validation** - `91174fc` (feat)
2. **Task 2: Implement chatter profiles list and show commands** - `e39912c` (feat)

## Files Created/Modified
- `src/commands/clone.rs` - Full clone command: input validation, voice cloning via bridge, profile saving, MP3 encoding
- `src/commands/profiles.rs` - Profiles list (formatted table), show (full details with file sizes), delete (with confirmation)

## Decisions Made
- WAV validation uses hound to check duration and sample rate; MP3 validation limited to file size > 1000 bytes (no MP3 decoder available)
- Clone saves source audio path as canonicalized absolute path in profile metadata
- Model variant resolved from detected_backend(): mlx gets 0.6B variant, others get 1.7B CustomVoice
- Profiles delete uses simple stdin read_line for confirmation rather than adding a prompt library dependency

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered
None

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- Clone and profiles commands are fully implemented and compiling
- Design command (02-02) and generate command (02-04) are the remaining Phase 2 work
- All profile storage APIs are now exercised by both clone and profiles commands

---
*Phase: 02-voice-profiles-and-speech-generation*
*Completed: 2026-03-27*
