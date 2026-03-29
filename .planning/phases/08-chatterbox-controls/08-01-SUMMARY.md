---
phase: 08-chatterbox-controls
plan: 01
subsystem: cli
tags: [chatterbox, pyo3, tts, cli-flags]

requires:
  - phase: 06-engine-abstraction
    provides: ChatterBox engine stub with generate_speech API
provides:
  - Parameterized exaggeration and cfg_weight through full stack (Python engine, dispatcher, Rust bridge, CLI)
affects: [08-02, generate-command]

tech-stack:
  added: []
  patterns: [engine-parameter-forwarding through PyO3 bridge]

key-files:
  created: []
  modified:
    - chatter_bridge/engines/chatterbox.py
    - chatter_bridge/__init__.py
    - chatter_bridge/engines/qwen.py
    - src/cli.rs
    - src/bridge/inference.rs
    - src/commands/generate.rs

key-decisions:
  - "Qwen engine generate_speech accepts exaggeration/cfg_weight params for positional arg compatibility with dispatcher"

patterns-established:
  - "Engine parameters forwarded positionally through dispatcher to all engine backends"

requirements-completed: [FT-01]

duration: 3min
completed: 2026-03-29
---

# Phase 08 Plan 01: ChatterBox Controls - Exaggeration & CFG Wiring Summary

**Wired exaggeration and cfg_weight parameters through full stack: Python ChatterBox engine, dispatcher, Rust PyO3 bridge, and CLI --exaggeration/--cfg flags with backward-compatible 0.5 defaults**

## Performance

- **Duration:** 3 min
- **Started:** 2026-03-29T22:12:05Z
- **Completed:** 2026-03-29T22:14:55Z
- **Tasks:** 2
- **Files modified:** 6

## Accomplishments
- Python ChatterBox engine and dispatcher accept exaggeration=0.5 and cfg_weight=0.5 parameters
- Rust PyO3 bridge passes exaggeration and cfg_weight through to Python generate_speech
- CLI exposes --exaggeration and --cfg flags on the generate subcommand as Option<f64>
- All layers default to 0.5, preserving existing behavior

## Task Commits

Each task was committed atomically:

1. **Task 1: Add exaggeration and cfg_weight parameters to Python engine and dispatcher** - `78b696c` (feat)
2. **Task 2: Add --exaggeration and --cfg CLI flags and wire through Rust PyO3 bridge** - `cb7ebab` (feat)

## Files Created/Modified
- `chatter_bridge/engines/chatterbox.py` - Added exaggeration/cfg_weight params to generate_speech signature
- `chatter_bridge/__init__.py` - Dispatcher forwards exaggeration/cfg_weight to active engine
- `chatter_bridge/engines/qwen.py` - Accepts exaggeration/cfg_weight for positional arg compatibility
- `src/cli.rs` - Added --exaggeration and --cfg fields to GenerateArgs
- `src/bridge/inference.rs` - generate_speech accepts and passes exaggeration/cfg_weight through PyO3
- `src/commands/generate.rs` - Resolves Option values to defaults and passes to bridge

## Decisions Made
- Updated qwen.py generate_speech signature to accept exaggeration/cfg_weight for positional argument compatibility with the dispatcher (deviation Rule 1 - would crash at runtime otherwise)

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Added exaggeration/cfg_weight params to qwen.py engine**
- **Found during:** Task 1 (Python engine updates)
- **Issue:** Dispatcher passes exaggeration and cfg_weight as positional args, but qwen.py generate_speech only accepted 6 params -- would crash with TypeError at runtime
- **Fix:** Added exaggeration=0.5 and cfg_weight=0.5 to qwen.py generate_speech signature
- **Files modified:** chatter_bridge/engines/qwen.py
- **Verification:** Python syntax check passes
- **Committed in:** 78b696c (Task 1 commit)

---

**Total deviations:** 1 auto-fixed (1 bug)
**Impact on plan:** Essential for runtime correctness. No scope creep.

## Issues Encountered
None

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- Exaggeration/cfg parameters wired through all layers, ready for Plan 02 engine-gating validation
- Plan 02 will add validation (error for --exaggeration with qwen engine, warning for non-Original variants)

---
*Phase: 08-chatterbox-controls*
*Completed: 2026-03-29*

## Self-Check: PASSED
