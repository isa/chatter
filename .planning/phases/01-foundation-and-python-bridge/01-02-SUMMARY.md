---
phase: 01-foundation-and-python-bridge
plan: 02
subsystem: bridge
tags: [pyo3, python, huggingface-hub, torch, mlx, indicatif, owo-colors]

requires:
  - phase: 01-foundation-and-python-bridge/01
    provides: "CLI scaffold with Cargo.toml, clap types, stub command handlers"
provides:
  - "PyO3 bridge module (error types, runtime detection, model management)"
  - "Real model download/list/remove via huggingface_hub"
  - "Compute backend detection (CUDA > MLX > MPS > CPU)"
  - "Progress spinner pattern for blocking Python operations"
affects: [01-foundation-and-python-bridge/03, 02-voice-profile-engine]

tech-stack:
  added: [pyo3 0.28 (auto-initialize), thiserror 2.x, indicatif 0.18, owo-colors 4.x]
  patterns: [PyO3 Python::attach for GIL acquisition, PyIterator for Python collection iteration, BridgeError for typed Python errors]

key-files:
  created:
    - src/bridge/mod.rs
    - src/bridge/error.rs
    - src/bridge/runtime.rs
    - src/bridge/model.rs
  modified:
    - src/commands/model.rs
    - src/main.rs

key-decisions:
  - "Used PyIterator::from_object for iterating Python sets (huggingface_hub repos/revisions are frozensets)"
  - "Used .str()?.extract() for repo_path since it may be a pathlib.Path not a string"
  - "Separated import_hf_hub helper to DRY the huggingface_hub import error handling across functions"
  - "Used red-only (not bold) for error prefix due to owo-colors if_supports_color lifetime constraints"

patterns-established:
  - "Bridge error pattern: BridgeError enum with thiserror, From<PyErr>, and domain-specific variants"
  - "Python import guard: check PyModuleNotFoundError to convert to friendly error messages"
  - "Spinner pattern: create_spinner() helper with cyan spinner, message, and elapsed time"
  - "Backend detection: try_detect_X functions returning Option to cascade through priorities"

requirements-completed: [FOUN-01, UX-01]

duration: 4min
completed: 2026-03-27
---

# Phase 1 Plan 2: PyO3 Bridge and Model Commands Summary

**PyO3 Python bridge with compute backend detection (CUDA/MLX/MPS/CPU), HuggingFace model management, and spinner-equipped model subcommands**

## Performance

- **Duration:** 4 min
- **Started:** 2026-03-27T13:07:37Z
- **Completed:** 2026-03-27T13:12:06Z
- **Tasks:** 2
- **Files modified:** 6

## Accomplishments
- Built complete PyO3 bridge module with typed errors, runtime detection, and model operations
- Wired model download/list/remove commands to real PyO3 calls through huggingface_hub
- Added progress spinners with elapsed time for blocking Python operations
- Established compute backend detection supporting CUDA, MLX, MPS, and CPU

## Task Commits

Each task was committed atomically:

1. **Task 1: Create PyO3 bridge module** - `e1822d0` (feat)
2. **Task 2: Wire model commands with spinners** - `b3fdecc` (feat)

## Files Created/Modified
- `src/bridge/mod.rs` - Bridge module re-exports (BridgeError, ComputeBackend, model functions)
- `src/bridge/error.rs` - BridgeError enum with Python, QwenTtsNotInstalled, PythonNotFound, NoGpuAvailable, ModelNotFound, Other variants
- `src/bridge/runtime.rs` - ComputeBackend enum and detect_backend() with CUDA > MLX > MPS > CPU priority
- `src/bridge/model.rs` - download_model, list_cached_models, remove_model via huggingface_hub PyO3 calls
- `src/commands/model.rs` - Model subcommands with real bridge calls, spinners, colored errors, formatted table output
- `src/main.rs` - Added `mod bridge` declaration

## Decisions Made
- Used `PyIterator::from_object()` instead of `.iter()` for Python set iteration (huggingface_hub cache repos are frozensets)
- Extracted `import_hf_hub()` helper to centralize the import-with-friendly-error pattern across all model functions
- Used `.str()?.extract()` for `repo_path` to handle Python pathlib.Path objects correctly
- Used red-only error prefix instead of red+bold due to owo-colors `if_supports_color` closure lifetime constraints with chained style methods

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Fixed PyO3 iteration over Python collections**
- **Found during:** Task 1 (bridge module)
- **Issue:** `.iter()` does not exist on `Bound<PyAny>` -- Python sets/frozensets need explicit iterator creation
- **Fix:** Used `pyo3::types::PyIterator::from_object()` with explicit `Bound<'_, PyAny>` type annotations
- **Files modified:** src/bridge/model.rs
- **Verification:** cargo check passes
- **Committed in:** e1822d0

**2. [Rule 1 - Bug] Fixed owo-colors if_supports_color lifetime error**
- **Found during:** Task 2 (model commands)
- **Issue:** Chaining `.red().bold()` inside `if_supports_color` closure creates a temporary that cannot be returned
- **Fix:** Used `.red()` only (dropped `.bold()`) to avoid the lifetime issue
- **Files modified:** src/commands/model.rs
- **Verification:** cargo build passes
- **Committed in:** b3fdecc

---

**Total deviations:** 2 auto-fixed (1 blocking, 1 bug)
**Impact on plan:** Both fixes necessary for compilation. No scope creep.

## Issues Encountered
- Plan 01 (Wave 1) artifacts did not exist in this worktree -- cherry-picked commits from parallel agent's worktree before starting

## User Setup Required
None - no external service configuration required.

## Known Stubs
None - all model commands are wired to real PyO3 bridge calls.

## Next Phase Readiness
- Bridge module ready for Plan 03 (doctor command, TTS inference)
- detect_backend() available for doctor command and model loading device selection
- Model download/list/remove fully operational through huggingface_hub

## Self-Check: PASSED

All 6 files verified present. Both task commits (e1822d0, b3fdecc) verified in git history.

---
*Phase: 01-foundation-and-python-bridge*
*Completed: 2026-03-27*
