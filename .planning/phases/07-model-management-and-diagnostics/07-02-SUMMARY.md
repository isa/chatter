---
phase: 07-model-management-and-diagnostics
plan: 02
subsystem: cli
tags: [doctor, diagnostics, chatterbox, multi-engine]

# Dependency graph
requires:
  - phase: 04-engine-abstraction
    provides: Engine enum and multi-engine architecture
provides:
  - ChatterBox diagnostic checks in doctor command
  - Extended --fix to install ChatterBox deps alongside Qwen
  - list_cached_chatterbox_models() for ChatterBox model discovery
affects: [08-voice-clone-and-generation]

# Tech tracking
tech-stack:
  added: []
  patterns: [informational-warning-for-optional-engine, per-engine-section-headers]

key-files:
  created: []
  modified:
    - src/bridge/doctor.rs
    - src/commands/doctor.rs
    - src/bridge/model.rs
    - src/bridge/mod.rs

key-decisions:
  - "ChatterBox not-installed shown as warning (not failure) -- optional engine per D-03"
  - "Used pip subprocess for ChatterBox install in --fix rather than waiting for bridge functions from Plan 01"
  - "Added list_cached_chatterbox_models() to bridge/model.rs filtering by ResembleAI/chatterbox repo_id pattern"

patterns-established:
  - "Per-engine section headers in doctor output (bold labels: Qwen3-TTS, ChatterBox)"
  - "Optional engine checks use doctor_warn not doctor_fail"

requirements-completed: [MDL-02]

# Metrics
duration: 3min
completed: 2026-03-29
---

# Phase 07 Plan 02: Doctor ChatterBox Diagnostics Summary

**Doctor command validates ChatterBox installation alongside Qwen3-TTS with per-engine sections and extended --fix for both engines**

## Performance

- **Duration:** 3 min
- **Started:** 2026-03-29T21:43:28Z
- **Completed:** 2026-03-29T21:46:46Z
- **Tasks:** 2
- **Files modified:** 4

## Accomplishments
- SystemInfo extended with chatterbox_pkg_version and chatterbox_installed fields
- Doctor output now shows separate Qwen3-TTS and ChatterBox sections with bold headers
- ChatterBox not-installed state displayed as informational warning (not failure)
- --fix installs ChatterBox deps via pip and handles errors gracefully (optional engine)
- Existing Qwen model checks relabeled as "Qwen Models" for clarity

## Task Commits

Each task was committed atomically:

1. **Task 1: Add ChatterBox fields to SystemInfo and get_system_info** - `cf59ba4` (feat)
2. **Task 2: Add ChatterBox section to doctor output and extend --fix** - `d46086a` (feat)

## Files Created/Modified
- `src/bridge/doctor.rs` - Added chatterbox_pkg_version and chatterbox_installed fields to SystemInfo
- `src/commands/doctor.rs` - Added ChatterBox section, per-engine headers, extended --fix handler
- `src/bridge/model.rs` - Added list_cached_chatterbox_models() for ChatterBox model discovery
- `src/bridge/mod.rs` - Exported list_cached_chatterbox_models

## Decisions Made
- ChatterBox not-installed shown as warning (not failure) per D-03 -- optional engine should not penalize users who only use Qwen
- Used pip subprocess for ChatterBox install in --fix rather than depending on bridge::venv::install_chatterbox_deps() which doesn't exist yet (Plan 01 parallel execution)
- Added list_cached_chatterbox_models() as separate function rather than modifying existing list_cached_models() to avoid breaking Qwen-specific callers

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Created list_cached_chatterbox_models() instead of using Plan 01 functions**
- **Found during:** Task 2 (ChatterBox section in doctor)
- **Issue:** Plan referenced bridge::venv::install_chatterbox_deps() and bridge::model::download_model_chatterbox() which don't exist (Plan 01 runs in parallel)
- **Fix:** Added list_cached_chatterbox_models() to bridge/model.rs for model discovery; used pip subprocess for --fix installation
- **Files modified:** src/bridge/model.rs, src/bridge/mod.rs, src/commands/doctor.rs
- **Verification:** cargo check and cargo clippy pass with no errors
- **Committed in:** d46086a (Task 2 commit)

---

**Total deviations:** 1 auto-fixed (1 blocking)
**Impact on plan:** Necessary adaptation due to parallel plan execution. Functions can be unified when Plan 01 completes.

## Issues Encountered
None beyond the deviation documented above.

## User Setup Required
None - no external service configuration required.

## Known Stubs
None - all functionality is wired to real data sources.

## Next Phase Readiness
- Doctor command ready for both engines
- When Plan 01 adds install_chatterbox_deps/download_model_chatterbox, the --fix handler could be updated to use those instead of direct pip subprocess

---
*Phase: 07-model-management-and-diagnostics*
*Completed: 2026-03-29*
