---
phase: 02-voice-profiles-and-speech-generation
verified: 2026-03-28T00:00:00Z
status: human_needed
score: 5/5 must-haves verified
human_verification:
  - test: "Run `chatter design 'warm friendly male voice'` end-to-end"
    expected: "Profile saved in ~/.config/chatter/profiles/ with sample.mp3 and voice_prompt.bin (or ref_audio.wav on MLX)"
    why_human: "Requires GPU and qwen-tts model — cannot run in CI"
  - test: "Run `chatter clone reference.mp3` with a real MP3 file"
    expected: "Profile saved with sample.mp3 and voice_prompt.bin; language stored as full English name (e.g. 'English'), not short code"
    why_human: "Requires GPU and qwen-tts; also surfaces language inconsistency behavior"
  - test: "Run `chatter generate 'Hello world' --profile myvoice` with a previously designed profile"
    expected: "MP3 file written to current directory with name like myvoice-20260328-120000.mp3; file is audible speech"
    why_human: "Requires GPU, model, and a real profile on disk"
  - test: "Run `chatter profiles list` after creating at least one profile"
    expected: "Table shows name, type, language, and creation date columns with correct values"
    why_human: "Requires profiles on disk from prior end-to-end runs"
  - test: "Run `chatter generate 'Hello' --profile myvoice --language english` to verify GEN-06"
    expected: "Speech generated using English language regardless of profile's stored language"
    why_human: "Requires GPU and real profile; tests language override logic"
---

# Phase 2: Voice Profiles and Speech Generation — Verification Report

**Phase Goal:** Users can create reusable voice profiles (by description or cloning) and generate speech from inline text using those profiles
**Verified:** 2026-03-28
**Status:** human_needed
**Re-verification:** No — initial verification

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|---------|
| 1 | User can run `chatter design` with a natural language description and get a saved voice profile with cached sample audio | VERIFIED | `src/commands/design.rs` 177 lines — full interactive loop: spinner, inference call, MP3 preview, accept/retry prompt, sample.mp3 encoding, TOML save |
| 2 | User can run `chatter clone` with a reference MP3 and get a saved voice profile with cached sample audio | VERIFIED | `src/commands/clone.rs` 222 lines — validation, bridge call, profile dir creation, sample.mp3 encoding, TOML save |
| 3 | User can run `chatter profiles list` and see all saved profiles with name, type, language, and creation date | VERIFIED | `src/commands/profiles.rs` `run_list()` — calls `storage::list_profiles()`, renders formatted table with dynamic column widths |
| 4 | User can run `chatter generate "some text" --profile myvoice` and get an MP3 file of spoken audio | VERIFIED | `src/commands/generate.rs` 137 lines — loads profile, resolves language (GEN-06), resolves output path, spinner, inference, MP3 encode, optional playback |
| 5 | User sees progress bars during voice profile creation and speech synthesis | VERIFIED | Spinner in `design.rs` line 52, `clone.rs` line 26, `generate.rs` line 108 — all use `ui::create_spinner()` and call `finish_and_clear()` |

**Score:** 5/5 truths verified (code level). Human verification required for runtime behavior.

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `src/profile/mod.rs` | ProfileMetadata, ProfileType, TOML serde types | VERIFIED | 45 lines; exports ProfileMetadata, ProfileInfo, AudioInfo, ProfileType with full Serialize/Deserialize derives |
| `src/profile/storage.rs` | Profile directory CRUD | VERIFIED | 91 lines; exports save_profile, load_profile, list_profiles, profiles_dir, slugify, unique_profile_name, profile_dir |
| `src/audio/mod.rs` | WAV-to-MP3 encoding pipeline | VERIFIED | 46 lines; exports encode_wav_to_mp3, samples_f32_to_i16; uses mp3lame_encoder::Builder with encode_to_vec/flush_to_vec |
| `src/audio/playback.rs` | System audio playback | VERIFIED | 33 lines; exports play_audio; dispatches afplay (macOS) / paplay / aplay (Linux) |
| `src/bridge/inference.rs` | PyO3 calls to chatter_bridge.py | VERIFIED | 124 lines; exports voice_design, voice_clone_from_audio, generate_speech, create_and_save_clone_prompt, unload_all_models, detected_backend |
| `chatter_bridge.py` | Python adapter normalizing qwen-tts and mlx-audio APIs | VERIFIED | 216 lines; detect_backend function present; covers all inference paths for both MLX and CUDA/MPS backends |
| `src/commands/design.rs` | Full design command with interactive preview loop | VERIFIED | 177 lines (exceeds 80-line min); contains `fn run`; interactive loop implemented |
| `src/commands/clone.rs` | Full clone command with input validation | VERIFIED | 222 lines (exceeds 60-line min); contains `fn run`; validate_audio_file() and validate_wav() present |
| `src/commands/profiles.rs` | Profiles list and show commands | VERIFIED | 207 lines (exceeds 40-line min); contains `fn run`; dispatches List, Show, Delete |
| `src/commands/generate.rs` | Full generate command implementation | VERIFIED | 137 lines (exceeds 60-line min); contains `fn run` |

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| `src/profile/storage.rs` | `src/profile/mod.rs` | uses ProfileMetadata for TOML serialization | VERIFIED | `use super::ProfileMetadata` at line 5; `toml::to_string_pretty(profile)` at line 21 |
| `src/bridge/inference.rs` | `chatter_bridge.py` | PyO3 import and call_method | VERIFIED | `py.import("chatter_bridge")` at line 114; all functions use `call_method1` / `call_method0` |
| `src/audio/mod.rs` | mp3lame_encoder | crate dependency | VERIFIED | `use mp3lame_encoder::{Builder, FlushNoGap, MonoPcm}` at line 3; `Builder::new()`, `encode_to_vec`, `flush_to_vec` used |
| `src/commands/design.rs` | `src/bridge/inference.rs` | bridge::voice_design() | VERIFIED | `inference::voice_design(...)` at line 54; `inference::create_and_save_clone_prompt(...)` at line 129 |
| `src/commands/design.rs` | `src/profile/storage.rs` | save_profile, slugify, unique_profile_name | VERIFIED | `storage::slugify(...)` line 35; `storage::unique_profile_name(...)` line 36; `storage::save_profile(...)` line 164 |
| `src/commands/design.rs` | `src/audio/mod.rs` | encode_wav_to_mp3, samples_f32_to_i16 | VERIFIED | `audio::samples_f32_to_i16(...)` lines 72, 134; `audio::encode_wav_to_mp3(...)` lines 73, 135 |
| `src/commands/design.rs` | `src/audio/playback.rs` | play_audio for preview | VERIFIED | `audio::playback::play_audio(&temp_mp3)` at line 79 |
| `src/commands/clone.rs` | `src/bridge/inference.rs` | bridge::voice_clone_from_audio() | VERIFIED | `bridge::voice_clone_from_audio(...)` at line 29; `bridge::create_and_save_clone_prompt(...)` at line 48 |
| `src/commands/clone.rs` | `src/profile/storage.rs` | save_profile, slugify, unique_profile_name | VERIFIED | `storage::profile_dir(...)` line 43; `storage::save_profile(...)` line 95 |
| `src/commands/profiles.rs` | `src/profile/storage.rs` | list_profiles, load_profile | VERIFIED | `storage::list_profiles()` at line 20; `storage::load_profile(name)` at line 85 |
| `src/commands/generate.rs` | `src/bridge/inference.rs` | bridge::generate_speech() | VERIFIED | `inference::generate_speech(...)` at line 111 |
| `src/commands/generate.rs` | `src/profile/storage.rs` | load_profile, profile_dir | VERIFIED | `storage::load_profile(...)` at line 56; `storage::profile_dir(...)` at line 64 |
| `src/commands/generate.rs` | `src/audio/mod.rs` | encode_wav_to_mp3, samples_f32_to_i16 | VERIFIED | `audio::samples_f32_to_i16(...)` line 119; `audio::encode_wav_to_mp3(...)` line 120 |
| `src/main.rs` | all commands | dispatches all 6 subcommands | VERIFIED | `Commands::Design`, `Clone`, `Generate`, `Profiles`, `Model`, `Doctor` all dispatched in match |

### Data-Flow Trace (Level 4)

This is a CLI tool with GPU inference — data flows from Python model through PyO3 to Rust encoding and file output. No UI rendering; all data paths are write-to-file, not display-from-state.

| Artifact | Data Variable | Source | Produces Real Data | Status |
|----------|--------------|--------|---------------------|--------|
| `chatter_bridge.py::voice_design` | wav samples | `model.generate_voice_design(...)` via qwen-tts or mlx-audio | Yes — actual model inference call | FLOWING |
| `chatter_bridge.py::generate_speech` | wav samples | `model.generate_voice_clone(...)` via qwen-tts or mlx-audio | Yes — uses saved voice_prompt.bin or ref_audio.wav | FLOWING |
| `src/audio/mod.rs::encode_wav_to_mp3` | PCM samples | WAV float32 from inference result | Yes — encodes real PCM | FLOWING |
| `src/profile/storage.rs::list_profiles` | profiles vec | reads TOML files from `~/.config/chatter/profiles/` | Yes — real filesystem reads | FLOWING |

### Behavioral Spot-Checks

Step 7b: SKIPPED — inference requires GPU and model weights; no runnable entry points available without full environment setup.

Compilation check performed as proxy:

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| Binary compiles without errors | `cargo check` | "Finished dev profile" — 15 warnings (dead code), 0 errors | PASS |
| All 6 commits from summaries exist in git log | `git log` | d9da3e7, 197b8bd, 15b5315, 91174fc, e39912c, 3376b61 all present | PASS |
| ModelSize enum fully removed | `grep -r ModelSize src/` | No matches | PASS |
| Profile storage path correct | `profiles_dir()` in storage.rs | Uses `ProjectDirs::from("","","chatter").config_dir().join("profiles")` — resolves to `~/.config/chatter/profiles/` on Linux / `~/Library/Application Support/chatter/profiles/` on macOS | PASS |

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|------------|-------------|--------|---------|
| PROF-01 | 02-02 | User can create voice profile from natural language description via `chatter design` | SATISFIED | `design.rs::run()` — full interactive workflow with inference + profile save |
| PROF-02 | 02-03 | User can create voice profile from reference MP3 file via `chatter clone` | SATISFIED | `clone.rs::run()` — validation + bridge call + profile save |
| PROF-03 | 02-01 | Voice profiles saved to `~/.config/chatter/profiles/` with metadata (TOML) and cached sample audio (MP3) | SATISFIED | `storage::save_profile()` writes to `profiles_dir().join(name)/profile.toml`; design.rs and clone.rs both write `sample.mp3` |
| PROF-04 | 02-03 | User can list all saved voice profiles via `chatter profiles list` | SATISFIED | `profiles.rs::run_list()` — calls `list_profiles()` and renders formatted table |
| PROF-05 | 02-01 | Profile metadata includes: name, type, language, description/source, creation date | SATISFIED | `ProfileInfo` struct has name, profile_type, language, description, source_audio, created, model_variant |
| PROF-06 | 02-02 | Cached sample audio generated at profile creation time for previewing | SATISFIED | design.rs line 132-136 encodes `sample.mp3` from accepted wav; clone.rs line 59-60 encodes `sample.mp3` from cloned wav |
| GEN-01 | 02-04 | User can generate speech from inline text using a saved voice profile via `chatter generate` | SATISFIED | `generate.rs::run()` — loads profile, calls `inference::generate_speech()`, encodes MP3 |
| GEN-05 | 02-04 | Generated audio saved as MP3 to user-specified or default output path | SATISFIED | Default path: `{profile}-{timestamp}.mp3` in CWD; `--output` flag supported |
| GEN-06 | 02-04 | Language flag on generate overrides profile's default language when specified | SATISFIED | generate.rs lines 77-84: CLI `--language` (when not Auto) takes precedence; profile language used as fallback |
| UX-02 | 02-04 | Progress bar displays during speech synthesis | SATISFIED | `create_spinner("Generating speech...")` at generate.rs line 108 |
| UX-03 | 02-02, 02-03 | Progress bar displays during voice profile creation (design and clone) | SATISFIED | Spinner in design.rs line 52, clone.rs line 26 |

**All 11 required Phase 2 requirements (PROF-01 through PROF-06, GEN-01, GEN-05, GEN-06, UX-02, UX-03) are satisfied.**

No orphaned requirements: REQUIREMENTS.md traceability table assigns only the 11 listed IDs to Phase 2.

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| `src/commands/clone.rs` | 210-221 | `language_to_string()` returns full English names ("Chinese", "English") while `design.rs` and `generate.rs` use short codes ("zh", "en") | Warning | Clone voice profiles store full English names in TOML; generate.rs then passes the stored name directly to the Python bridge. If qwen-tts only accepts one format, this could produce wrong language behavior at generation time for cloned profiles. |
| `src/bridge/mod.rs` | — | `detected_backend` not re-exported from bridge/mod.rs | Info | `design.rs` imports via `crate::bridge::inference` directly (bypassing pub re-exports), which works but is inconsistent with how other bridge functions are consumed. Not a bug. |
| `src/commands/generate.rs` | 41-53 | File input path returns early with "not yet supported" message | Info | Correct Phase 3 deferral; user gets a clear error. Not a stub — intentional boundary. |

### Human Verification Required

#### 1. End-to-End Design Command

**Test:** Run `chatter design "warm friendly male voice"` on a machine with GPU and qwen-tts installed
**Expected:** Spinner appears, voice preview plays via afplay/paplay, accept/retry prompt shown, profile saved to `~/.config/chatter/profiles/warm-friendly-male-voice/` containing profile.toml, sample.mp3, and voice_prompt.bin (or ref_audio.wav on MLX)
**Why human:** Requires GPU, model weights, and audio output device; cannot run without full environment

#### 2. End-to-End Clone Command — Language Storage Format

**Test:** Run `chatter clone reference.wav --language english`, then inspect the saved profile.toml
**Expected:** Profile created successfully; verify what language value is stored — "English" (full name from clone.rs) vs "en" (short code from design.rs)
**Why human:** This surfaces the language format inconsistency noted in anti-patterns; the stored value is then used as-is in `generate.rs` when language override is not provided

#### 3. Generate with Cloned Profile Language Passthrough

**Test:** Clone a profile with `--language english`, then run `chatter generate "Hello" --profile myvoice` without `--language` flag
**Expected:** Speech generated correctly in English — confirms qwen-tts accepts the full English name "English" passed from clone.rs
**Why human:** Requires GPU, models, and real profile; validates that language inconsistency between clone.rs and design.rs/generate.rs does not cause runtime failures

#### 4. Profiles List Table Rendering

**Test:** Run `chatter profiles list` after creating at least two profiles (one designed, one cloned)
**Expected:** Table shows all profiles with correct name, type, language, and YYYY-MM-DD creation date columns; unicode separator line renders correctly
**Why human:** Terminal rendering and column alignment cannot be verified from code inspection alone

#### 5. GEN-06 Language Override

**Test:** Design a profile with `--language auto`, then run `chatter generate "Hello" --profile myvoice --language english`
**Expected:** Speech is generated in English, not auto-detect mode
**Why human:** Requires real inference to confirm language override takes effect

### Gaps Summary

No blocking gaps found. All 5 observable truths are verified at the code level. All 11 requirements have implementation evidence. The codebase compiles cleanly (zero errors, 15 non-blocking dead-code warnings).

One warning-severity finding requires human verification:

**Language format inconsistency:** `clone.rs::language_to_string()` passes full English names ("Chinese", "English") to the Python bridge and stores them in profile TOML. `design.rs` and `generate.rs` use short ISO codes ("zh", "en"). When a cloned profile is used with `chatter generate` without `--language` override, `generate.rs` passes the stored language string (e.g., "English") directly to `inference::generate_speech()`, which passes it to qwen-tts. If qwen-tts does not accept "English" (only "en"), generation from cloned profiles would use the wrong language string. This needs runtime verification.

---

_Verified: 2026-03-28_
_Verifier: Claude (gsd-verifier)_
