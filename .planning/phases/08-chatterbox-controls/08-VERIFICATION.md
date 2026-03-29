---
phase: 08-chatterbox-controls
verified: 2026-03-29T22:30:00Z
status: passed
score: 9/9 must-haves verified
re_verification: false
---

# Phase 08: ChatterBox Controls Verification Report

**Phase Goal:** Users can leverage ChatterBox-specific audio generation features not available in Qwen3-TTS
**Verified:** 2026-03-29
**Status:** passed
**Re-verification:** No -- initial verification

---

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | Python chatterbox engine accepts exaggeration and cfg_weight as parameters instead of hardcoding 0.5 | VERIFIED | `_generate_with_model` and `generate_speech` in chatterbox.py both accept `exaggeration=0.5, cfg_weight=0.5`; Original variant passes them to `model.generate()` |
| 2 | Rust bridge passes exaggeration and cfg values through PyO3 to Python | VERIFIED | `inference::generate_speech` takes `exaggeration: f64, cfg_weight: f64`; `call_method1("generate_speech", (..., exaggeration, cfg_weight))` at line 127 |
| 3 | CLI exposes --exaggeration and --cfg flags on the generate subcommand | VERIFIED | `pub exaggeration: Option<f64>` and `pub cfg: Option<f64>` in `GenerateArgs` (cli.rs lines 210-214) |
| 4 | Using --exaggeration with --engine qwen produces a clear error | VERIFIED | `generate.rs` lines 152-159: bails with `"--exaggeration is only available with --engine chatterbox"` |
| 5 | Using --exaggeration with ChatterBox Turbo produces a warning but succeeds | VERIFIED | `generate.rs` lines 181-188: `eprintln!("Warning: --exaggeration has no effect with ChatterBox {} variant", effective_variant)` when `effective_variant != "original"` |
| 6 | User can pass --exaggeration 0.7 and hear different expressiveness with ChatterBox Original | VERIFIED (flow-level) | Parameters flow from CLI -> `args.exaggeration.unwrap_or(0.5)` -> both inference call sites -> PyO3 -> Python engine -> `model.generate(exaggeration=exaggeration, ...)` in Original branch. Perceptual difference requires human test. |
| 7 | Text containing [laugh] with ChatterBox Turbo passes validation | VERIFIED | `validate_paralinguistic_tags` accepts `[laugh]` (in VALID_TAGS); unit test `valid_tags_accepted` covers this case |
| 8 | Text containing [invalid_tag] with ChatterBox Turbo produces a clear validation error before inference | VERIFIED | `validate_paralinguistic_tags` returns `Err("Invalid paralinguistic tag(s): [invalid_tag]\nValid tags for ChatterBox Turbo: ...")` and `generate.rs` maps this to `anyhow::bail!` before model loading |
| 9 | Tags in text with Qwen engine are ignored (treated as literal text) | VERIFIED | Tag validation block in `generate.rs` lines 250-256 is gated to `Engine::Chatterbox || profile.profile.engine == "chatterbox"` AND `effective_variant == "turbo"` -- Qwen path never enters the validator |

**Score:** 9/9 truths verified

---

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `chatter_bridge/engines/chatterbox.py` | Parameterized exaggeration and cfg_weight in `_generate_with_model` and `generate_speech` | VERIFIED | Signature: `def _generate_with_model(model, backend, text, language, ref_audio_path, exaggeration=0.5, cfg_weight=0.5)` and `def generate_speech(..., exaggeration=0.5, cfg_weight=0.5)`. Original branch uses params, not hardcoded values. |
| `chatter_bridge/__init__.py` | Dispatcher routes exaggeration and cfg_weight to engine | VERIFIED | `def generate_speech(text, language, profile_dir, ref_text="", temperature=0.7, repetition_penalty=1.2, exaggeration=0.5, cfg_weight=0.5)` forwards all 8 args positionally to `_get_engine().generate_speech(...)` |
| `src/bridge/inference.rs` | PyO3 bridge passes exaggeration and cfg through to Python | VERIFIED | Function signature has `exaggeration: f64, cfg_weight: f64`; `call_method1` tuple includes both values (line 127) |
| `src/cli.rs` | --exaggeration and --cfg flags on GenerateArgs | VERIFIED | `pub exaggeration: Option<f64>` and `pub cfg: Option<f64>` with `#[arg(long)]` |
| `src/commands/validate.rs` | Paralinguistic tag validation function | VERIFIED | `pub fn validate_paralinguistic_tags(text: &str) -> Result<(), String>` with all 8 tags and 4 unit tests |
| `src/commands/generate.rs` | Engine-gated flag validation and tag validation before inference | VERIFIED | Engine gating (lines 152-159), variant warning (181-188), range validation (195-197), tag validation (249-256), both inference call sites pass real values (271-274, 282-285) |

---

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| `src/bridge/inference.rs` | `chatter_bridge/__init__.py` | PyO3 `call_method1` with exaggeration and cfg_weight | WIRED | `bridge.call_method1("generate_speech", (..., exaggeration, cfg_weight))` at inference.rs line 124-128 |
| `chatter_bridge/__init__.py` | `chatter_bridge/engines/chatterbox.py` | `generate_speech` forwards kwargs | WIRED | `_get_engine().generate_speech(text, language, profile_dir, ref_text, temperature, repetition_penalty, exaggeration, cfg_weight)` -- positional forwarding confirmed |
| `src/commands/generate.rs` | `src/commands/validate.rs` | `validate_paralinguistic_tags` call before inference | WIRED | `use crate::commands::validate;` at top; `validate::validate_paralinguistic_tags(&text)` at line 254, before `inference::ensure_model_loaded` at line 261 |
| `src/commands/generate.rs` | `src/bridge/inference.rs` | `generate_speech` call with exaggeration and cfg_weight | WIRED | Both single-chunk (line 271) and multi-chunk (line 282) call sites pass `exaggeration, cfg_weight` as final two args |

---

### Data-Flow Trace (Level 4)

| Artifact | Data Variable | Source | Produces Real Data | Status |
|----------|--------------|--------|-------------------|--------|
| `src/commands/generate.rs` | `exaggeration` | `args.exaggeration.unwrap_or(0.5)` (line 191) | Yes -- CLI-provided value or default | FLOWING |
| `src/commands/generate.rs` | `cfg_weight` | `args.cfg.unwrap_or(0.5)` (line 192) | Yes -- CLI-provided value or default | FLOWING |
| `chatter_bridge/engines/chatterbox.py` | `exaggeration`, `cfg_weight` | Received as function params, passed to `model.generate(exaggeration=exaggeration, cfg_weight=cfg_weight)` | Yes -- non-empty only when `_variant == "original"` branch executes | FLOWING |

No hollow props or disconnected data paths found.

---

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| `cargo check` passes with new parameters | `cargo check` | `Finished dev profile [unoptimized + debuginfo]` (9 warnings, 0 errors) | PASS |
| Python files parse without syntax errors | `python3 -c "import ast; ast.parse(...)"` | `Python syntax OK` | PASS |
| All 8 official tags present in validator | grep on validate.rs | `[chuckle] [cough] [cry] [gasp] [groan] [laugh] [sigh] [yawn]` | PASS |
| No hardcoded 0.5, 0.5 at inference call sites | grep on generate.rs | No matches | PASS |
| Commits documented in summaries exist in git log | `git log --oneline` | `78b696c`, `cb7ebab`, `d74e6ac`, `04940ba` all present | PASS |

Note: Perceptual audio difference for `--exaggeration 0.7` requires hardware inference -- see Human Verification below.

---

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|------------|-------------|--------|----------|
| FT-01 | 08-01, 08-02 | User can control emotion intensity via `--exaggeration` (0.0-1.0) and `--cfg` flags when generating with ChatterBox Original | SATISFIED | `--exaggeration` and `--cfg` flags wired through CLI -> Rust bridge -> Python dispatcher -> ChatterBox engine Original branch. Range validated 0.0-1.0. Error for Qwen engine. Warning for non-Original variants. |
| FT-02 | 08-02 | User can use paralinguistic tags (`[laugh]`, `[sigh]`, etc.) in text input for ChatterBox Turbo, with syntax validation | SATISFIED | `validate_paralinguistic_tags` in `src/commands/validate.rs` checks all 8 official tags before inference. Only fires for ChatterBox Turbo variant. Invalid tags produce clear error with valid tag list. |

No orphaned requirements found. Both FT-01 and FT-02 (the only requirements mapped to Phase 08 in REQUIREMENTS.md) are fully claimed and implemented.

---

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| `chatter_bridge/engines/chatterbox.py` | 224-225 (MLX branch) | `_generate_with_model` MLX path ignores `exaggeration` and `cfg_weight` | Info | Expected behavior -- MLX ChatterBox models expose a different API with no equivalent params. The accepted params are silently unused in the MLX path, which is consistent with the Turbo/Multilingual PyTorch paths. Not a stub. |

No blockers. No functional stubs. No placeholder comments. No hardcoded empty returns. The one informational note (MLX ignoring exaggeration) is architecturally correct -- ChatterBox MLX community models do not expose these generation controls.

---

### Human Verification Required

#### 1. Perceptual Exaggeration Effect

**Test:** Run `chatter --engine chatterbox generate "Hello, I'm so excited about this!" --profile <cb-profile> --exaggeration 0.9` and compare to the same command with `--exaggeration 0.1`
**Expected:** The 0.9 version should sound noticeably more emotionally expressive / intense compared to the 0.1 version
**Why human:** Requires GPU hardware, a real ChatterBox voice profile, and subjective audio comparison. Cannot verify perceptual audio difference programmatically.

#### 2. Paralinguistic Tag Audio Effect

**Test:** Run `chatter --engine chatterbox generate "I can't believe it [laugh] that's amazing [sigh]" --profile <cb-turbo-profile>` with a Turbo variant profile
**Expected:** The generated MP3 should contain audible non-speech sounds at `[laugh]` and `[sigh]` positions
**Why human:** Requires GPU hardware and listening to the output audio. Cannot programmatically verify that ChatterBox Turbo actually renders the paralinguistic sounds.

#### 3. Engine Gating Error at Runtime

**Test:** Run `chatter generate "Hello" --profile <qwen-profile> --exaggeration 0.7`
**Expected:** CLI exits with error `"--exaggeration is only available with --engine chatterbox"` before attempting inference
**Why human:** Can be tested without GPU (early bail before model load), but needs a real Qwen profile to load for the engine check to trigger. The code path is verified; this is a smoke test.

---

## Gaps Summary

No gaps. All 9 observable truths verified. Both FT-01 and FT-02 requirements satisfied. Rust compiles cleanly. Python files parse without errors. Key links are fully wired with data flowing through all layers.

The phase goal -- "Users can leverage ChatterBox-specific audio generation features not available in Qwen3-TTS" -- is achieved:
- Emotion/expressiveness control via `--exaggeration` and `--cfg` (FT-01)
- Paralinguistic tag syntax with pre-inference validation (FT-02)
- Engine-gated errors and variant-gated warnings prevent user confusion

One notable deviation from Plan 01 (qwen.py also accepting exaggeration/cfg_weight params for positional arg compatibility with the dispatcher) was auto-fixed and is correct by design -- the dispatcher calls all engines with the same positional argument list.

---

_Verified: 2026-03-29_
_Verifier: Claude (gsd-verifier)_
