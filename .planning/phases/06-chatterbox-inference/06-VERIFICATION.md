---
phase: 06-chatterbox-inference
verified: 2026-03-29T21:30:00Z
status: passed
score: 11/11 must-haves verified
re_verification: false
---

# Phase 06: ChatterBox Inference Verification Report

**Phase Goal:** Users can clone voices and generate speech using ChatterBox models
**Verified:** 2026-03-29T21:30:00Z
**Status:** passed
**Re-verification:** No — initial verification

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | Running `chatter model download --engine chatterbox` installs ChatterBox deps into managed venv | VERIFIED | `src/commands/model.rs:53-73` dispatches to `install_chatterbox_deps()` + `download_model_chatterbox()` on `Engine::Chatterbox` |
| 2 | When ChatterBox deps not installed, a clear error directs user to `chatter model download --engine chatterbox` | VERIFIED | `chatter_bridge/engines/chatterbox.py:43-52` — `_check_deps()` raises `ImportError` with exact install instructions; called at entry points for `detect_backend`, `generate_speech`, `voice_clone_from_audio`, `ensure_model`, `load_base_model` |
| 3 | ChatterBox dep installation does not break existing Qwen3-TTS functionality | VERIFIED | `requirements/chatterbox.txt` explicitly excludes `torch`/`torchaudio` (already in venv); no changes to `chatter_bridge/engines/qwen.py` |
| 4 | `--cb-variant` flag available on clone and generate subcommands | VERIFIED | `src/cli.rs:164` (`CloneArgs`) and `src/cli.rs:206` (`GenerateArgs`) both have `pub cb_variant: Option<ChatterBoxVariant>` |
| 5 | When `--engine chatterbox` is used, clone.rs and generate.rs call `bridge::set_variant()` before inference | VERIFIED | `src/commands/clone.rs:42-49`: gated on `Engine::Chatterbox`; `src/commands/generate.rs:151-160`: gated on chatterbox engine |
| 6 | ChatterBox profiles persist the `cb_variant` field in profile metadata | VERIFIED | `src/profile/mod.rs:28` — `pub cb_variant: Option<String>` with `#[serde(skip_serializing_if = "Option::is_none")]`; `src/commands/clone.rs:224` saves `cb_variant: cb_variant_str` |
| 7 | User can clone a voice from audio using ChatterBox via the existing clone workflow | VERIFIED | `chatter_bridge/engines/chatterbox.py:272-283` — `voice_clone_from_audio` fully implemented; MLX and PyTorch paths both present |
| 8 | User can generate speech from text using a ChatterBox voice profile | VERIFIED | `chatter_bridge/engines/chatterbox.py:252-269` — `generate_speech` reads `ref_audio.wav` from `profile_dir`, handles all 3 variants |
| 9 | Switching from qwen to chatterbox engine unloads qwen models before loading chatterbox | VERIFIED | `chatter_bridge/__init__.py:95-99` — `set_engine()` calls `_active_engine.unload_all_models()` before switching, wrapped in try/except |
| 10 | ChatterBox detect_backend returns mlx on Apple Silicon (except multilingual), cuda/mps/cpu as fallback | VERIFIED | `chatter_bridge/engines/chatterbox.py:98-133` — `if _variant != "multilingual"` guards MLX check; full fallback chain present |
| 11 | MLX path uses mlx-audio community models for Original and Turbo variants | VERIFIED | `chatter_bridge/engines/chatterbox.py:55-65` — `_mlx_model_id()` returns `mlx-community/chatterbox-fp16` (original) and `mlx-community/chatterbox-turbo-fp16` (turbo); raises ValueError for multilingual |

**Score:** 11/11 truths verified

---

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `requirements/chatterbox.txt` | Curated dep list, contains `chatterbox-tts==0.1.7` | VERIFIED | 12 lines, contains `chatterbox-tts==0.1.7`, no `gradio`/`torch`/`torchaudio`/`resemble-perth` |
| `src/bridge/venv.rs` | `install_chatterbox_deps()` function | VERIFIED | Lines 216-283: full pip install pipeline with `--no-deps`, curated requirements, and mlx-audio on Apple Silicon |
| `src/bridge/model.rs` | Engine-aware download dispatching | VERIFIED | `chatterbox_model_variants()` (L63), `download_model_chatterbox()` (L192), `remove_chatterbox_models()` (L212), `list_cached_models()` detects chatterbox repos (L156-159) |
| `src/commands/model.rs` | Engine-aware download command handler | VERIFIED | `Engine::Chatterbox` match arm at L53 dispatches to install + download |
| `src/cli.rs` | `ChatterBoxVariant` enum and `--cb-variant` flag on clone/generate | VERIFIED | Enum at L64-83 with `as_str()` impl; `cb_variant` on both `CloneArgs` (L164) and `GenerateArgs` (L206) |
| `src/bridge/inference.rs` | `set_variant()` PyO3 bridge function | VERIFIED | Lines 17-25: `pub fn set_variant(variant: &str)` calls `bridge.call_method1("set_variant", (variant,))` |
| `src/profile/mod.rs` | `cb_variant` field on `ProfileInfo` | VERIFIED | L28: `pub cb_variant: Option<String>` with `#[serde(skip_serializing_if = "Option::is_none")]` |
| `src/commands/clone.rs` | Calls `bridge::set_variant()` for ChatterBox and saves `cb_variant` | VERIFIED | L42-49: set_variant call gated on `Engine::Chatterbox`; L224: `cb_variant: cb_variant_str` in ProfileInfo construction |
| `src/commands/generate.rs` | Calls `inference::set_variant()` for ChatterBox engine | VERIFIED | L151-160: set_variant with profile fallback for variant string |
| `chatter_bridge/engines/chatterbox.py` | Full ChatterBox engine (>200 lines, all key functions) | VERIFIED | 326 lines; exports: `detect_backend`, `generate_speech`, `voice_clone_from_audio`, `unload_all_models`, `ensure_model`, `is_model_loaded`, `set_variant` |
| `chatter_bridge/__init__.py` | Engine-switch memory cleanup in `set_engine()` + `set_variant` dispatcher | VERIFIED | `set_engine()` unloads previous engine (L95-99); `set_variant()` function at L174-181 |

---

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| `src/commands/model.rs` | `src/bridge/venv.rs` | `install_chatterbox_deps()` call | WIRED | `bridge::venv::install_chatterbox_deps()` called at L56 |
| `src/bridge/venv.rs` | `requirements/chatterbox.txt` | `CHATTERBOX_REQUIREMENTS` const | WIRED | `const CHATTERBOX_REQUIREMENTS: &str = include_str!("../../requirements/chatterbox.txt")` at L22; used at L238 |
| `src/commands/clone.rs` | `src/bridge/inference.rs` | `set_variant()` call before inference | WIRED | `bridge::set_variant(variant.as_str())` at L44 |
| `src/commands/generate.rs` | `src/bridge/inference.rs` | `set_variant()` call before inference | WIRED | `inference::set_variant(&variant_str)` at L158 |
| `src/bridge/inference.rs` | `chatter_bridge/__init__.py` | PyO3 `call_method1` for `set_variant` | WIRED | `bridge.call_method1("set_variant", (variant,))` at L24 |
| `chatter_bridge/__init__.py` | `chatter_bridge/engines/chatterbox.py` | `set_engine` dispatches to chatterbox | WIRED | `AVAILABLE_ENGINES["chatterbox"] = "chatter_bridge.engines.chatterbox"` in `engines/__init__.py` |
| `chatter_bridge/engines/chatterbox.py` | `mlx_audio.tts` | MLX model loading | WIRED | `from mlx_audio.tts.utils import load_model` at L179 (runtime conditional) |
| `chatter_bridge/engines/chatterbox.py` | `chatterbox.tts` | PyTorch model loading | WIRED | `from chatterbox.tts import ChatterboxTTS` at L83; turbo/multilingual variants also imported |
| `chatter_bridge/__init__.py set_engine()` | `engine.unload_all_models()` | Memory cleanup before engine switch | WIRED | `_active_engine.unload_all_models()` at L97 |

---

### Data-Flow Trace (Level 4)

| Artifact | Data Variable | Source | Produces Real Data | Status |
|----------|---------------|--------|--------------------|--------|
| `chatterbox.py generate_speech` | `audio` (f32 list) | `_generate_with_model()` -> `model.generate()` | Yes — live inference from loaded model | FLOWING |
| `chatterbox.py voice_clone_from_audio` | `audio` (f32 list) | `_generate_with_model()` -> `model.generate()` | Yes — live inference from loaded model | FLOWING |
| `inference.rs generate_speech` | `(wav, sr)` | PyO3 call to Python `generate_speech` | Yes — data extracted from Python return value | FLOWING |
| `clone.rs` ProfileMetadata | `cb_variant_str` | Derived from `args.cb_variant` or defaulted | Yes — set from CLI flag or default | FLOWING |
| `generate.rs` variant_str | `variant_str` | CLI arg -> profile fallback -> hardcoded default | Yes — profile `cb_variant` read from JSON | FLOWING |

---

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| chatterbox.py Python syntax valid | `python3 -c "import ast; ast.parse(...)"` | Syntax OK | PASS |
| __init__.py Python syntax valid | `python3 -c "import ast; ast.parse(...)"` | Syntax OK | PASS |
| chatterbox.py >200 lines | `wc -l chatterbox.py` | 326 lines | PASS |
| Cargo compilation | `cargo check` | `Finished dev profile` (9 warnings, 0 errors) | PASS |
| Commit hashes documented in SUMMARY exist | `git log --oneline` | `578edfc`, `6143765` both present | PASS |

---

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|-------------|-------------|--------|----------|
| CB-01 | 06-02-PLAN.md | User can clone voice from audio using ChatterBox | SATISFIED | `chatterbox.py:voice_clone_from_audio()` fully implemented; `clone.rs` calls it via bridge for ChatterBox engine |
| CB-02 | 06-02-PLAN.md | User can generate speech from text using ChatterBox voice profile | SATISFIED | `chatterbox.py:generate_speech()` reads `ref_audio.wav` from profile_dir; `generate.rs` calls it via bridge |
| CB-03 | 06-02-PLAN.md | Switching engines automatically unloads previous engine models | SATISFIED | `chatter_bridge/__init__.py:set_engine()` calls `_active_engine.unload_all_models()` with try/except before switching |
| CB-04 | 06-01-PLAN.md | ChatterBox Python deps resolve without breaking Qwen3-TTS | SATISFIED | `requirements/chatterbox.txt` excludes torch/torchaudio (already in venv); curated `--no-deps` install prevents conflicts |

No orphaned requirements: CB-01, CB-02, CB-03, CB-04 are the only Phase 06 requirements in REQUIREMENTS.md. All are covered by the two plans.

---

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| `src/bridge/venv.rs` | 286 | `is_chatterbox_installed()` defined but never called | Info | No functional impact — the check is done via Python `_check_deps()` which raises an ImportError through PyO3. The Rust function exists for future use (e.g., doctor command in Phase 07). |
| `src/bridge/error.rs` | 50 | `ChatterBoxNotInstalled` variant defined but never constructed in Rust code | Info | No functional impact — the user-facing error message is equivalent, delivered via Python ImportError through PyO3 as `BridgeError::Python`. The Rust variant is available for Phase 07 doctor command. |

No blockers or warnings. Both noted patterns are forward-looking scaffolding, not stubs — the actual functionality works through the Python `_check_deps()` path.

---

### Human Verification Required

#### 1. Voice cloning produces recognizable audio on Apple Silicon (MLX path)

**Test:** Run `chatter --engine chatterbox clone reference.wav --name testvoice` on an Apple Silicon Mac with mlx-audio installed
**Expected:** Completes without error, creates a profile directory with `ref_audio.wav`, `sample.mp3`, and `profile.json` containing `"engine": "chatterbox"` and `"cb_variant": "original"`
**Why human:** Requires actual ChatterBox deps installed in venv and Apple Silicon hardware. Cannot verify inference output quality programmatically.

#### 2. Voice cloning produces recognizable audio on PyTorch MPS path

**Test:** Run `chatter --engine chatterbox clone reference.wav` on Apple Silicon when mlx-audio is not installed (forcing MPS fallback)
**Expected:** Loads model using `from_pretrained("cpu")` + selective `.to("mps")` submodel transfer, generates audio without MPS crash
**Why human:** Requires real MPS hardware and chatterbox-tts installed. The CPU-first loading pattern is code-verified but runtime MPS compatibility requires physical testing.

#### 3. Engine switch frees memory before loading new engine

**Test:** Run `chatter --engine qwen generate "text" --profile qwen-voice` (loads Qwen), then run `chatter --engine chatterbox clone ref.wav` immediately after in same session
**Expected:** Memory usage does not spike to combined total of both engines; no OOM on 16GB Mac
**Why human:** Memory profiling requires physical hardware. The code correctness of `unload_all_models()` + `gc.collect()` + device cache clear is verified, but actual memory behavior requires runtime observation.

#### 4. Clear error message when ChatterBox not installed

**Test:** Run `chatter --engine chatterbox clone ref.wav` without running `chatter model download --engine chatterbox` first
**Expected:** Error message displays: "ChatterBox is not installed. Run: chatter model download --engine chatterbox to install ChatterBox models and dependencies."
**Why human:** Requires a real environment where chatterbox-tts is not installed in the venv. The error text is in the Python `_check_deps()` function but needs end-to-end testing to confirm it surfaces correctly through PyO3 and Rust error handling.

---

### Gaps Summary

No gaps found. All 11 observable truths are verified. All 4 phase requirements (CB-01, CB-02, CB-03, CB-04) have implementation evidence. The two noted items (`is_chatterbox_installed` unused, `ChatterBoxNotInstalled` unraised) are non-blocking forward scaffolding — they do not break any goal or requirement.

The implementation is complete and correct at the code level. Human verification items cover runtime behavior on real hardware that cannot be tested programmatically.

---

_Verified: 2026-03-29T21:30:00Z_
_Verifier: Claude (gsd-verifier)_
