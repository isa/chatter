---
phase: 07-model-management-and-diagnostics
plan: 01
subsystem: cli
tags: [model-management, disk-space, chatterbox, owo-colors]

requires:
  - phase: 05-cli-engine-routing
    provides: Engine enum and --engine global flag
provides:
  - Engine-grouped model list output (Qwen3-TTS / ChatterBox sections)
  - ChatterBox variant labels (Original, Turbo, Multilingual)
  - disk_space_check() for pre-download space verification
  - Disk space warning on ChatterBox download when less than 5GB remains
affects: [07-model-management-and-diagnostics, 06-inference-pipeline]

tech-stack:
  added: []
  patterns: [engine-grouped CLI output, disk space pre-check before large downloads]

key-files:
  created: []
  modified:
    - src/bridge/model.rs
    - src/commands/model.rs

key-decisions:
  - "Hardcoded 20GB estimate for ChatterBox download size since actual sizes depend on variant selection"
  - "ChatterBox download path returns early with informational message since download_model_chatterbox() is not yet implemented"
  - "disk_space_check() failure is non-blocking -- warns and proceeds"

patterns-established:
  - "Engine-grouped output: partition models by engine field, print section headers with bold styling"
  - "Pre-download disk space check: call disk_space_check() before large downloads, warn if tight"

requirements-completed: [MDL-01, MDL-03]

duration: 3min
completed: 2026-03-29
---

# Phase 7 Plan 1: Model Management Enhancement Summary

**Engine-grouped model listing with ChatterBox variant labels and disk space pre-check before large downloads**

## Performance

- **Duration:** 3 min
- **Started:** 2026-03-29T21:43:17Z
- **Completed:** 2026-03-29T21:46:02Z
- **Tasks:** 2
- **Files modified:** 2

## Accomplishments
- ModelInfo struct extended with engine and variant_label fields for engine-aware model management
- Model list output now groups models under "Qwen3-TTS Models" and "ChatterBox Models" section headers
- ChatterBox models display variant labels (Original, Turbo, Multilingual) alongside repo IDs
- Disk space pre-check added to ChatterBox download path showing estimated size and available space with low-space warning

## Task Commits

Each task was committed atomically:

1. **Task 1: Add disk space check and engine/variant fields to bridge model layer** - `68ecd28` (feat)
2. **Task 2: Engine-grouped model list and disk space pre-check in command layer** - `d39d49c` (feat)

## Files Created/Modified
- `src/bridge/model.rs` - Added engine/variant_label fields to ModelInfo, disk_space_check() function, chatterbox_variant_label() helper, updated list_cached_models() for ChatterBox detection
- `src/commands/model.rs` - Engine-grouped list output with section headers, disk space pre-check before ChatterBox download with yellow warning

## Decisions Made
- Hardcoded 20GB estimate for ChatterBox download size since actual sizes depend on which variants are selected
- ChatterBox download path returns early with informational message since the actual download function is not yet implemented (will be wired in inference pipeline phase)
- disk_space_check() failure is non-blocking to avoid preventing downloads when shutil is unavailable

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] ChatterBox download function does not exist yet**
- **Found during:** Task 2 (command layer)
- **Issue:** Plan references `install_chatterbox_deps()` and `download_model_chatterbox()` which are not implemented yet
- **Fix:** Added disk space pre-check as planned but added an early return with informational message after the check since there is no ChatterBox download to invoke
- **Files modified:** src/commands/model.rs
- **Verification:** cargo check passes
- **Committed in:** d39d49c (Task 2 commit)

---

**Total deviations:** 1 auto-fixed (1 blocking)
**Impact on plan:** Disk space pre-check infrastructure is in place and will activate when ChatterBox download is wired. No scope creep.

## Issues Encountered
None

## User Setup Required
None - no external service configuration required.

## Known Stubs
- `src/commands/model.rs` line in ChatterBox download branch: prints "ChatterBox model download is not yet available." and returns early. This is intentional -- actual download will be implemented in the inference pipeline phase (06).

## Next Phase Readiness
- Engine-grouped model list ready for use
- disk_space_check() ready to be called before ChatterBox download once download_model_chatterbox() is implemented
- Variant labels will display correctly once ChatterBox models are cached

## Self-Check: PASSED

All files and commits verified.

---
*Phase: 07-model-management-and-diagnostics*
*Completed: 2026-03-29*
