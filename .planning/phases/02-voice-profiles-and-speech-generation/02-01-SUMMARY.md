---
phase: 02-voice-profiles-and-speech-generation
plan: 01
subsystem: bridge, profile, audio
tags: [pyo3, mp3lame, toml, mlx, qwen-tts, voice-profiles, audio-encoding]

# Dependency graph
requires:
  - phase: 01-rust-tts-foundation
    provides: CLI skeleton, bridge module (runtime, venv, model, error), ui module
provides:
  - Profile types (ProfileMetadata, ProfileInfo, AudioInfo, ProfileType) with TOML serde
  - Profile storage CRUD (save, load, list, slugify, unique_profile_name)
  - WAV-to-MP3 encoding pipeline via mp3lame-encoder
  - System audio playback (afplay/paplay/aplay)
  - Python bridge adapter (chatter_bridge.py) normalizing qwen-tts and mlx-audio APIs
  - Rust inference module (voice_design, generate_speech, create_and_save_clone_prompt, voice_clone_from_audio)
  - Backend-aware venv setup (mlx-audio on Apple Silicon, qwen-tts elsewhere)
  - CLI without ModelSize (always 1.7B), GenerateArgs with --play flag
affects: [02-02, 02-03, 02-04]

# Tech tracking
tech-stack:
  added: [toml 0.8, hound 3.5, mp3lame-encoder 0.2, chrono 0.4]
  patterns: [backend-aware model variant selection, embedded Python adapter via include_str, MonoPcm encoding with encode_to_vec]

key-files:
  created:
    - src/profile/mod.rs
    - src/profile/storage.rs
    - src/audio/mod.rs
    - src/audio/playback.rs
    - src/bridge/inference.rs
    - chatter_bridge.py
  modified:
    - Cargo.toml
    - src/cli.rs
    - src/main.rs
    - src/bridge/mod.rs
    - src/bridge/model.rs
    - src/bridge/error.rs
    - src/bridge/venv.rs
    - src/commands/model.rs

key-decisions:
  - "Removed ModelSize enum entirely -- always use 1.7B models (D-01 decision)"
  - "Backend-aware venv: install mlx-audio on macOS ARM64, qwen-tts elsewhere"
  - "Embed chatter_bridge.py via include_str! and write to site-packages at venv creation"
  - "Use encode_to_vec/flush_to_vec for mp3lame-encoder (API uses MaybeUninit buffers)"

patterns-established:
  - "Profile storage: TOML files in ~/.config/chatter/profiles/<name>/profile.toml"
  - "Python bridge pattern: Rust import_bridge() -> call_method on module functions"
  - "Backend detection cascading: chatter_bridge.py caches backend, returns mlx/cuda/mps/cpu"

requirements-completed: [PROF-03, PROF-05]

# Metrics
duration: 6min
completed: 2026-03-27
---

# Phase 2 Plan 1: Foundation Modules Summary

**Profile types with TOML storage, WAV-to-MP3 encoding via mp3lame, Python bridge adapter normalizing qwen-tts and mlx-audio APIs, and CLI cleanup removing ModelSize**

## Performance

- **Duration:** 6 min
- **Started:** 2026-03-27T18:16:39Z
- **Completed:** 2026-03-27T18:23:13Z
- **Tasks:** 2
- **Files modified:** 15

## Accomplishments
- Profile module with types (ProfileMetadata, ProfileInfo, AudioInfo, ProfileType) and full CRUD storage operations
- Audio module with float32-to-i16 conversion and MP3 encoding via mp3lame-encoder, plus system playback
- Python bridge adapter (chatter_bridge.py) covering all inference paths: voice design, clone prompt creation, speech generation, and direct clone-from-audio
- Rust inference module wrapping all Python bridge functions via PyO3
- CLI cleaned up: ModelSize enum removed, --play flag added, model commands simplified to always-1.7B
- Bridge model module updated to serve MLX or PyTorch variants based on detected backend
- Venv setup is now backend-aware and auto-installs chatter_bridge.py into site-packages

## Task Commits

Each task was committed atomically:

1. **Task 1: Add crate dependencies, clean up CLI, create profile and audio modules** - `d9da3e7` (feat)
2. **Task 2: Create Python bridge adapter and Rust inference module** - `197b8bd` (feat)

## Files Created/Modified
- `src/profile/mod.rs` - ProfileMetadata, ProfileInfo, AudioInfo, ProfileType types with TOML serde
- `src/profile/storage.rs` - Profile directory CRUD: save, load, list, slugify, unique_profile_name
- `src/audio/mod.rs` - WAV-to-MP3 encoding pipeline using mp3lame-encoder
- `src/audio/playback.rs` - System audio playback (afplay on macOS, paplay/aplay on Linux)
- `src/bridge/inference.rs` - Rust PyO3 wrappers for chatter_bridge.py functions
- `chatter_bridge.py` - Python adapter normalizing qwen-tts and mlx-audio model APIs
- `Cargo.toml` - Added toml, hound, mp3lame-encoder, chrono dependencies
- `src/cli.rs` - Removed ModelSize enum, added --play flag to GenerateArgs
- `src/main.rs` - Added mod audio and mod profile declarations
- `src/bridge/mod.rs` - Added pub mod inference with re-exports
- `src/bridge/model.rs` - Refactored to backend-aware variants (MLX vs PyTorch), removed ModelSize parameter
- `src/bridge/error.rs` - Added VoiceDesignFailed, VoiceCloneFailed, GenerationFailed, AudioEncodingFailed, ProfileError, BackendNotAvailable variants
- `src/bridge/venv.rs` - Backend-aware packages, embed/install chatter_bridge.py, check bridge importability
- `src/commands/model.rs` - Updated to work without ModelSize parameter

## Decisions Made
- Removed ModelSize enum entirely per D-01 -- all commands now assume 1.7B models
- Backend-aware venv: installs mlx-audio on macOS ARM64, qwen-tts on other platforms
- Embedded chatter_bridge.py source via `include_str!` and writes to site-packages during venv creation
- Used mp3lame-encoder's `encode_to_vec`/`flush_to_vec` convenience methods (raw API uses MaybeUninit buffers incompatible with Vec<u8>)

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Fixed mp3lame-encoder API usage**
- **Found during:** Task 1 (audio module creation)
- **Issue:** Plan's mp3lame-encoder code used method chaining with `.expect()` which doesn't work because `set_*` methods return `Result<(), _>` via `&mut self` (not builder pattern). Also used raw `encode`/`flush` with `&mut [u8]` but API requires `&mut [MaybeUninit<u8>]`.
- **Fix:** Used mutable builder pattern, switched to `encode_to_vec`/`flush_to_vec` convenience methods
- **Files modified:** src/audio/mod.rs
- **Verification:** cargo check passes
- **Committed in:** d9da3e7 (Task 1 commit)

---

**Total deviations:** 1 auto-fixed (1 bug fix)
**Impact on plan:** API usage fix necessary for compilation. No scope creep.

## Issues Encountered
None beyond the mp3lame-encoder API mismatch documented above.

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- All foundation modules compile and are ready for downstream plans (02-02 design command, 02-03 clone command, 02-04 generate command)
- Profile types, storage, audio encoding, bridge inference are all importable
- chatter_bridge.py covers all inference paths needed by command implementations

---
*Phase: 02-voice-profiles-and-speech-generation*
*Completed: 2026-03-27*
