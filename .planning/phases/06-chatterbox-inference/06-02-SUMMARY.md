---
phase: 06-chatterbox-inference
plan: 02
subsystem: inference
tags: [chatterbox, tts, mlx, pytorch, voice-cloning, engine-switching]

# Dependency graph
requires:
  - phase: 06-chatterbox-inference/01
    provides: "Rust bridge stubs, venv management, CLI variant flag, Python stub module"
provides:
  - "Full ChatterBox engine: detect_backend, generate_speech, voice_clone_from_audio, unload_all_models"
  - "Memory-safe engine switching in dispatcher (unload before switch)"
  - "set_variant dispatcher function for ChatterBox variant selection"
affects: [06-chatterbox-inference/03, 07-model-management, 08-voice-features]

# Tech tracking
tech-stack:
  added: [chatterbox-tts, mlx-audio]
  patterns: [mlx-first-backend-detection, cpu-first-pytorch-loading, variant-aware-inference, engine-memory-cleanup]

key-files:
  created: []
  modified:
    - chatter_bridge/engines/chatterbox.py
    - chatter_bridge/__init__.py

key-decisions:
  - "Extracted shared generation logic into _generate_with_model helper to avoid duplication between generate_speech and voice_clone_from_audio"
  - "Wrapped PyTorch model loading in _suppress_warnings_only (not _suppress_output) so HF download progress remains visible"

patterns-established:
  - "MPS-safe loading: always from_pretrained('cpu') then selective .to(device) for submodels"
  - "Backend cache reset on variant change (MLX availability depends on variant)"
  - "Engine unload before switch: try/except best-effort cleanup"

requirements-completed: [CB-01, CB-02, CB-03]

# Metrics
duration: 3min
completed: 2026-03-29
---

# Phase 06 Plan 02: ChatterBox Engine Implementation Summary

**Full ChatterBox engine with MLX-first backend detection, variant-aware inference (Original/Turbo/Multilingual), MPS-safe PyTorch loading, and memory-safe engine switching**

## Performance

- **Duration:** 3 min
- **Started:** 2026-03-29T21:02:56Z
- **Completed:** 2026-03-29T21:05:34Z
- **Tasks:** 2
- **Files modified:** 2

## Accomplishments
- Replaced ChatterBox stub (66 lines, all NotImplementedError) with full 326-line engine implementation
- MLX-first backend detection that correctly skips MLX for multilingual variant (no community model)
- PyTorch MPS-safe loading pattern: CPU-first then selective submodel device transfer
- Memory-safe engine switching: dispatcher unloads previous engine models before loading new engine

## Task Commits

Each task was committed atomically:

1. **Task 1: Implement ChatterBox engine module** - `578edfc` (feat)
2. **Task 2: Add memory-safe engine switching to dispatcher** - `6143765` (feat)

## Files Created/Modified
- `chatter_bridge/engines/chatterbox.py` - Full ChatterBox engine: backend detection, model loading, variant-aware speech generation, voice cloning, memory cleanup
- `chatter_bridge/__init__.py` - Memory-safe set_engine() with unload before switch, new set_variant() dispatcher

## Decisions Made
- Extracted shared generation logic into `_generate_with_model()` helper to eliminate code duplication between `generate_speech` and `voice_clone_from_audio`
- Used `_suppress_warnings_only` for PyTorch model loading so HF download progress bars remain visible to users

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered
None

## User Setup Required
None - no external service configuration required.

## Known Stubs
None - all functions are fully implemented (voice_design and load_design_model intentionally raise NotImplementedError as ChatterBox does not support voice design).

## Next Phase Readiness
- ChatterBox engine fully functional for clone and generate commands
- Engine switching safely unloads models to prevent OOM on 16GB Macs
- Ready for Phase 06 Plan 03 (integration testing / remaining work)
- Ready for Phase 07 (model management) and Phase 08 (voice features)

---
*Phase: 06-chatterbox-inference*
*Completed: 2026-03-29*
