---
phase: 06-chatterbox-inference
plan: 01
subsystem: bridge
tags: [chatterbox, pyo3, venv, cli, tts]

# Dependency graph
requires:
  - phase: 05-cli-engine-routing
    provides: Engine enum, --engine global flag, set_engine bridge call
provides:
  - Curated ChatterBox requirements file (requirements/chatterbox.txt)
  - install_chatterbox_deps() venv function for --no-deps install pipeline
  - is_chatterbox_installed() check function
  - ChatterBoxNotInstalled error variant
  - Engine-aware model download/remove commands
  - ChatterBoxVariant enum (Original, Turbo, Multilingual) with --cb-variant CLI flag
  - set_variant() PyO3 bridge function
  - cb_variant field on ProfileInfo for profile persistence
  - Clone and generate commands call set_variant before ChatterBox inference
affects: [06-chatterbox-inference plan 02, 07-model-management, 08-chatterbox-features]

# Tech tracking
tech-stack:
  added: []
  patterns: [engine-aware command dispatch, curated pip requirements with --no-deps]

key-files:
  created:
    - requirements/chatterbox.txt
  modified:
    - src/bridge/venv.rs
    - src/bridge/error.rs
    - src/bridge/inference.rs
    - src/bridge/model.rs
    - src/cli.rs
    - src/commands/model.rs
    - src/commands/clone.rs
    - src/commands/generate.rs
    - src/profile/mod.rs
    - src/commands/design.rs

key-decisions:
  - "ChatterBox deps installed with --no-deps plus curated requirements to avoid gradio and resemble-perth bloat"
  - "mlx-audio installed only on Apple Silicon (cfg! compile-time check)"
  - "cb_variant uses Option<String> with serde skip_serializing_if for backward compat"

patterns-established:
  - "Engine-aware dispatch: match global.engine in command handlers for Qwen vs ChatterBox paths"
  - "Curated requirements: embed requirements file via include_str! for reproducible installs"

requirements-completed: [CB-04]

# Metrics
duration: 5min
completed: 2026-03-29
---

# Phase 06 Plan 01: ChatterBox Inference Infrastructure Summary

**ChatterBox dependency pipeline, engine-aware model download, variant CLI flag, set_variant bridge, and profile variant persistence**

## Performance

- **Duration:** 5 min
- **Started:** 2026-03-29T20:54:11Z
- **Completed:** 2026-03-29T20:59:28Z
- **Tasks:** 3
- **Files modified:** 10

## Accomplishments
- Created curated requirements/chatterbox.txt excluding gradio (~200MB) and resemble-perth (git URL)
- Added install_chatterbox_deps() with --no-deps install pipeline and Apple Silicon mlx-audio support
- Engine-aware model download/list/remove commands dispatch correctly for Qwen vs ChatterBox
- ChatterBoxVariant enum (Original, Turbo, Multilingual) with --cb-variant flag on clone and generate
- set_variant() PyO3 bridge function wired into clone and generate command flows
- cb_variant field persisted in profile metadata with backward-compatible serde defaults

## Task Commits

Each task was committed atomically:

1. **Task 1: Create curated requirements file and venv installation function** - `972257e` (feat)
2. **Task 2: Add engine-aware model download command and ChatterBox variant CLI flag** - `5362a03` (feat)
3. **Task 3: Bridge set_variant via PyO3, wire clone/generate commands, persist variant in profile** - `d57458e` (feat)

## Files Created/Modified
- `requirements/chatterbox.txt` - Curated ChatterBox dependency list (no gradio/torch)
- `src/bridge/venv.rs` - install_chatterbox_deps() and is_chatterbox_installed() functions
- `src/bridge/error.rs` - ChatterBoxNotInstalled error variant
- `src/bridge/inference.rs` - set_variant() PyO3 bridge function
- `src/bridge/model.rs` - download_model_chatterbox(), remove_chatterbox_models(), chatterbox_model_variants()
- `src/cli.rs` - ChatterBoxVariant enum, --cb-variant on CloneArgs and GenerateArgs
- `src/commands/model.rs` - Engine-aware download/remove dispatch
- `src/commands/clone.rs` - set_variant call, engine-aware model_variant, cb_variant persistence
- `src/commands/generate.rs` - set_variant call with CLI flag and profile fallback
- `src/profile/mod.rs` - cb_variant: Option<String> on ProfileInfo
- `src/commands/design.rs` - cb_variant: None for Qwen design profiles

## Decisions Made
- ChatterBox deps installed with --no-deps plus curated requirements to avoid gradio and resemble-perth bloat
- mlx-audio installed only on Apple Silicon via cfg! compile-time detection
- cb_variant stored as Option<String> with serde skip_serializing_if for backward compatibility

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Added cb_variant: None to design.rs ProfileInfo construction**
- **Found during:** Task 3 (adding cb_variant field to ProfileInfo)
- **Issue:** Adding a non-optional-without-default field to ProfileInfo would break compilation in design.rs
- **Fix:** Added `cb_variant: None` to the ProfileInfo construction in design.rs
- **Files modified:** src/commands/design.rs
- **Verification:** cargo check passes
- **Committed in:** d57458e (Task 3 commit)

---

**Total deviations:** 1 auto-fixed (1 bug)
**Impact on plan:** Essential for compilation. No scope creep.

## Issues Encountered
None

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- Full Rust-side infrastructure ready for ChatterBox Python engine module (Plan 06-02)
- set_variant bridge function ready to call Python set_variant method
- Profile metadata can store and retrieve cb_variant for ChatterBox profiles
- Model download pipeline ready to install ChatterBox deps and download HF models

---
*Phase: 06-chatterbox-inference*
*Completed: 2026-03-29*
