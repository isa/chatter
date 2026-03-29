---
phase: 08-chatterbox-controls
plan: 02
subsystem: cli
tags: [chatterbox, validation, cli-flags, paralinguistic]

requires:
  - phase: 08-chatterbox-controls
    plan: 01
    provides: exaggeration and cfg_weight CLI flags and bridge wiring
provides:
  - Engine-gated flag validation for --exaggeration and --cfg
  - Paralinguistic tag validation for ChatterBox Turbo
affects:
  - src/commands/generate.rs

tech_stack:
  added: []
  patterns:
    - Engine-specific flag gating with bail! errors
    - Variant-specific warnings via eprintln
    - Pre-inference text validation in Rust

key_files:
  created:
    - src/commands/validate.rs
  modified:
    - src/commands/generate.rs
    - src/commands/mod.rs

decisions:
  - Validate tags against original text before chunking to catch all invalid tags at once
  - Exaggeration range validated to 0.0-1.0 per D-03
  - Tag validation only fires for ChatterBox Turbo variant per D-06

metrics:
  duration: 3min
  completed: 2026-03-29
  tasks_completed: 2
  tasks_total: 2
  files_changed: 3
---

# Phase 08 Plan 02: Engine-Specific Flag Gating and Tag Validation Summary

Engine-gated --exaggeration/--cfg flags (error for Qwen, warning for non-Original variants) and paralinguistic tag validation for ChatterBox Turbo with 8 official tags.

## What Was Done

### Task 1: Engine-specific flag gating and exaggeration/cfg wiring
- Added bail! errors when --exaggeration or --cfg used with --engine qwen (D-07)
- Added eprintln warnings when --exaggeration or --cfg used with non-Original ChatterBox variants (D-08)
- Added exaggeration range validation 0.0-1.0 (D-03)
- Resolved exaggeration/cfg defaults once and passed to both inference call sites (single-chunk and multi-chunk)
- Removed inline `args.exaggeration.unwrap_or(0.5)` temporary defaults from Plan 01

### Task 2: Paralinguistic tag validation
- Created src/commands/validate.rs with `validate_paralinguistic_tags()` function
- Recognizes 8 official tags: [laugh], [chuckle], [cough], [sigh], [gasp], [groan], [yawn], [cry]
- Invalid tags produce clear error with the full valid tag list
- Validation gated to ChatterBox Turbo variant only (D-06)
- Wired validation before model loading in generate command
- Added unit tests covering valid, invalid, mixed, and unclosed bracket cases

## Commits

| Task | Commit | Message |
|------|--------|---------|
| 1 | d74e6ac | feat(08-02): add engine-specific flag gating and wire exaggeration/cfg in generate command |
| 2 | 04940ba | feat(08-02): create paralinguistic tag validation and wire into generate command |

## Verification

- `cargo check` passes with no errors
- `cargo build` succeeds
- All acceptance criteria verified via grep checks
- Unit tests for validate module compile (runtime blocked by PyO3 library linkage in test env, pre-existing)

## Deviations from Plan

None - plan executed exactly as written.

## Known Stubs

None - all functionality is fully wired.
