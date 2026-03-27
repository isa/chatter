---
phase: 01-foundation-and-python-bridge
plan: 03
subsystem: cli, diagnostics
tags: [pyo3, owo-colors, indicatif, doctor, diagnostics, no-color]

# Dependency graph
requires:
  - phase: 01-foundation-and-python-bridge/01
    provides: CLI scaffold with doctor stub command, GlobalArgs, Cargo.toml dependencies
provides:
  - Doctor command with pass/fail environment checklist
  - SystemInfo struct for gathering Python/GPU diagnostics via PyO3
  - Shared UI module (spinners, error formatting, doctor pass/fail/warn helpers)
  - BridgeError and ComputeBackend types (prerequisite for bridge module)
affects: [01-foundation-and-python-bridge/02, 02-voice-profiles-and-generation]

# Tech tracking
tech-stack:
  added: [pyo3 (Python::attach), owo-colors (if_supports_color), indicatif (spinner)]
  patterns: [single-GIL-acquisition for diagnostics, Option-per-check resilience, NO_COLOR support via Stream enum]

key-files:
  created:
    - src/bridge/doctor.rs
    - src/bridge/error.rs
    - src/bridge/runtime.rs
    - src/bridge/mod.rs
    - src/ui.rs
  modified:
    - src/commands/doctor.rs
    - src/main.rs

key-decisions:
  - "Used Rust std::fs for HF cache size walk instead of Python os.walk to avoid PyO3 CStr issues"
  - "Inlined backend detection logic via detect_backend_inner(py) to share GIL context"
  - "Used .to_string() pattern with if_supports_color to work around owo-colors lifetime constraints"

patterns-established:
  - "Single GIL pattern: gather all Python data in one Python::attach closure"
  - "Option-per-check: each diagnostic returns Option<T> so failures don't crash the report"
  - "UI helpers in src/ui.rs: doctor_pass/doctor_fail/doctor_warn for consistent formatting"
  - "NO_COLOR: all colored output uses if_supports_color(Stream::Stdout/Stderr, ...)"

requirements-completed: [FOUN-05, UX-01]

# Metrics
duration: 8min
completed: 2026-03-27
---

# Phase 1 Plan 3: Doctor Command Summary

**Doctor command with pass/fail checklist for Python, qwen-tts, PyTorch, GPU, and disk via single PyO3 GIL call, plus shared UI helpers for spinners and colored output**

## Performance

- **Duration:** 8 min
- **Started:** 2026-03-27T13:06:25Z
- **Completed:** 2026-03-27T13:14:02Z
- **Tasks:** 2
- **Files modified:** 7

## Accomplishments
- Doctor command gathers all system info in a single Python GIL acquisition
- Pass/fail checklist covers Python, qwen-tts, PyTorch, GPU backend (CUDA/MLX/MPS/CPU), and disk space
- Shared UI module provides reusable spinner, error formatting, and doctor output helpers
- Colors respect NO_COLOR env var via owo-colors if_supports_color
- Verbose mode shows HF cache path, backend details, and Python sys.path

## Task Commits

Each task was committed atomically:

1. **Task 1: Create doctor bridge module and shared UI helpers** - `a24c933` (feat)
2. **Task 2: Implement doctor command rendering and wire error presentation** - `4177740` (feat)

## Files Created/Modified
- `src/bridge/doctor.rs` - SystemInfo struct and get_system_info() gathering diagnostics via PyO3
- `src/bridge/error.rs` - BridgeError enum with Python, QwenTtsNotInstalled, NoGpuAvailable variants
- `src/bridge/runtime.rs` - ComputeBackend enum and detect_backend() for CUDA/MLX/MPS/CPU detection
- `src/bridge/mod.rs` - Bridge module with re-exports
- `src/ui.rs` - Shared UI: create_spinner, print_error, doctor_pass/fail/warn with NO_COLOR support
- `src/commands/doctor.rs` - Full doctor command rendering pass/fail checklist with summary
- `src/main.rs` - Added mod bridge and mod ui declarations

## Decisions Made
- Used Rust std::fs for HF cache size walking instead of Python os.walk (avoids CStr compatibility issues with PyO3 0.28's py.run)
- Inlined backend detection via detect_backend_inner(py) to share GIL context in doctor
- Used .to_string() pattern with owo-colors if_supports_color to work around lifetime constraints on chained color methods

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Created bridge prerequisite files (error.rs, runtime.rs)**
- **Found during:** Task 1
- **Issue:** Plan 02 (parallel Wave 2) creates bridge/error.rs and bridge/runtime.rs, but this worktree needed them to compile
- **Fix:** Created the prerequisite bridge files with types matching Plan 02 interface spec
- **Files modified:** src/bridge/error.rs, src/bridge/runtime.rs
- **Verification:** cargo check succeeds
- **Committed in:** a24c933 (Task 1 commit)

**2. [Rule 1 - Bug] Fixed PyO3 0.28 API compatibility**
- **Found during:** Task 1
- **Issue:** PyO3 0.28 methods (getattr, call_method0, etc.) require `use pyo3::prelude::*` to bring PyAnyMethods trait into scope. Also py.run expects &CStr not &str.
- **Fix:** Added pyo3::prelude::* imports, replaced Python os.walk with Rust std::fs for cache size calculation
- **Files modified:** src/bridge/runtime.rs, src/bridge/doctor.rs
- **Verification:** cargo check succeeds
- **Committed in:** a24c933 (Task 1 commit)

**3. [Rule 1 - Bug] Fixed owo-colors lifetime issue with chained color methods**
- **Found during:** Task 1
- **Issue:** `t.red().bold()` creates a temporary that can't be returned from the if_supports_color closure
- **Fix:** Used `t.red().to_string()` pattern which produces an owned String
- **Files modified:** src/ui.rs
- **Verification:** cargo check succeeds
- **Committed in:** a24c933 (Task 1 commit)

---

**Total deviations:** 3 auto-fixed (1 blocking, 2 bugs)
**Impact on plan:** All auto-fixes necessary for compilation. No scope creep. Bridge prerequisite files match Plan 02 interface spec and will be reconciled during merge.

## Issues Encountered
- Worktree did not have Plan 01 scaffolding initially; resolved by rebasing onto isa/rust-tts-qwen branch
- PyO3 0.28 API surface differs from older versions (renamed methods, CStr requirements); resolved by consulting error messages

## User Setup Required
None - no external service configuration required.

## Known Stubs
None - all functionality is fully wired.

## Next Phase Readiness
- Doctor command fully functional and tested
- Bridge module structure established for Plan 02's model operations
- UI helpers ready for reuse by model download spinners and error display

---
*Phase: 01-foundation-and-python-bridge*
*Completed: 2026-03-27*
