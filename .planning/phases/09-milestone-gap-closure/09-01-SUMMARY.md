---
phase: 09-milestone-gap-closure
plan: 01
subsystem: generate, doctor, bridge
tags: [engine-mismatch, tty-detection, dead-code, doctor-fix, chatterbox-install]
dependency_graph:
  requires: [08-01, 08-02]
  provides: [ENG-01, ENG-02, ENG-03, MDL-02]
  affects: [src/commands/generate.rs, src/commands/doctor.rs, src/bridge/venv.rs, src/bridge/error.rs]
tech_stack:
  added: []
  patterns: [std::io::IsTerminal, TTY-aware error handling, curated pip install pipeline]
key_files:
  created: []
  modified:
    - src/commands/generate.rs
    - src/commands/doctor.rs
    - src/bridge/venv.rs
    - src/bridge/error.rs
decisions:
  - TTY detection via std::io::IsTerminal on stdin (not stdout) — stdin is the correct stream to check for piped input
  - model download always runs after install (not gated by cb_models_missing) — fresh installs always need download
  - get_system_info() called again in --fix block to get fresh info for chatterbox_installed check
metrics:
  duration: 8min
  completed: 2026-03-30
  tasks_completed: 2
  files_modified: 4
---

# Phase 09 Plan 01: Milestone Gap Closure Summary

TTY-aware engine mismatch handling in generate plus curated ChatterBox install pipeline with hardware status in doctor.

## Tasks Completed

| Task | Description | Commit | Files |
|------|-------------|--------|-------|
| 1 | TTY-aware engine mismatch handling and dead code removal | 739069a | generate.rs, venv.rs, error.rs |
| 2 | Wire doctor --fix to curated install pipeline and add hardware check | 10dc073 | doctor.rs |

## What Was Built

### ENG-03: TTY-Aware Engine Mismatch (generate.rs)

Added `use std::io::IsTerminal` and wrapped the existing interactive engine switch prompt in `std::io::stdin().is_terminal()`. Added an `else` branch that immediately bails with a clear error message when running in piped/CI/non-interactive environments. Previously, the code would always attempt an interactive prompt, blocking indefinitely when stdin was not a terminal.

### MDL-02: Curated ChatterBox Install Pipeline (doctor.rs)

Replaced the local `install_chatterbox_via_pip()` function (which called plain `pip install chatterbox-tts`) with `bridge::venv::install_chatterbox_deps()` — the curated pipeline that uses `--no-deps` for chatterbox-tts itself, installs a pinned requirements file, and on Apple Silicon also installs `mlx-audio`. Model download now always runs after a successful install (previously gated by `cb_models_missing` which was false on first-time setup).

### Hardware Status in Doctor (doctor.rs)

Replaced raw `println!` hardware compatibility output with proper `ui::doctor_pass` / `ui::doctor_warn` calls. Added CPU and None cases that were previously missing. GPU passes now increment the `passes` counter consistently with the rest of the doctor output.

### Dead Code Removal

- `is_chatterbox_installed()` removed from `bridge/venv.rs` — was never called
- `ChatterBoxNotInstalled` variant removed from `bridge/error.rs` — was never constructed
- `get_system_info_chatterbox_installed()` removed from `doctor.rs` — replaced by `info.chatterbox_installed`
- `install_chatterbox_via_pip()` removed from `doctor.rs` — replaced by `bridge::venv::install_chatterbox_deps()`

## Requirements Satisfied

- **ENG-01**: Engine enum with Qwen/Chatterbox variants + CHATTER_ENGINE env var — confirmed already implemented in Phase 06 (src/cli.rs)
- **ENG-02**: Bridge dispatcher to qwen.py and chatterbox.py — confirmed already implemented in Phase 06 (chatter_bridge/__init__.py)
- **ENG-03**: Engine mismatch in non-interactive environments bails immediately with clear error
- **MDL-02**: Doctor --fix uses curated install pipeline with model download

## Deviations from Plan

### Auto-fixed Issues

None.

### Implementation Notes

1. The plan suggested moving the `--fix` block inside `if venv_ok` to gain access to `info`. Instead, we added a fresh `get_system_info()` call at the top of the `--fix` block (which already requires `venv_ok` to be true). This is equivalent but keeps the code structure unchanged and avoids restructuring the control flow.

## Known Stubs

None — all changes wire real functionality.

## Self-Check: PASSED

All modified files exist. All task commits (739069a, 10dc073) confirmed in git log.
