---
phase: 01-foundation-and-python-bridge
plan: 01
subsystem: cli
tags: [rust, clap, owo-colors, cli, pyo3]

# Dependency graph
requires: []
provides:
  - Compilable chatter binary with full CLI argument parsing
  - CLI type hierarchy (Cli, Commands, GlobalArgs, Language, ModelSize)
  - Stub command handlers for all 6 subcommands
  - Project scaffold (Cargo.toml, src/cli.rs, src/commands/*.rs)
affects: [01-02, 01-03, 02-voice-profiles, 02-speech-generation]

# Tech tracking
tech-stack:
  added: [clap 4.5, pyo3 0.28, indicatif 0.18, owo-colors 4, anyhow 1, thiserror 2, serde 1, serde_json 1, directories 6]
  patterns: [clap derive API, owo-colors Style + if_supports_color pattern, command module dispatch]

key-files:
  created: [Cargo.toml, src/cli.rs, src/main.rs, src/commands/mod.rs, src/commands/design.rs, src/commands/clone.rs, src/commands/generate.rs, src/commands/profiles.rs, src/commands/model.rs, src/commands/doctor.rs]
  modified: []

key-decisions:
  - "Used owo-colors Style builder with if_supports_color instead of chained method calls to avoid temporary reference lifetime issues"
  - "All dependencies from CLAUDE.md tech stack included in Cargo.toml from the start for incremental compilation benefit"

patterns-established:
  - "Command dispatch: main.rs matches Commands enum, delegates to commands::{module}::run()"
  - "Colored output: construct Style, use if_supports_color(Stream::Stderr) for NO_COLOR compliance"
  - "Global args: passed as &GlobalArgs reference to all command handlers"

requirements-completed: [FOUN-02, FOUN-03, FOUN-04, UX-04]

# Metrics
duration: 5min
completed: 2026-03-27
---

# Phase 1 Plan 1: Rust Project Scaffold and CLI Summary

**Rust CLI with clap derive API providing 6 subcommands, Language (11 variants) and ModelSize (2 variants) enum validation, and colored stub messages**

## Performance

- **Duration:** 5 min
- **Started:** 2026-03-27T12:53:19Z
- **Completed:** 2026-03-27T12:58:47Z
- **Tasks:** 2
- **Files modified:** 10

## Accomplishments
- Full CLI type hierarchy with all subcommands, nested commands, and argument structs
- Language enum validates 11 values, ModelSize validates 0.6b/1.7b with user-friendly names
- Global flags (--verbose, --language, --model-size) propagate to all subcommands
- Stub commands print colored "Phase 2" messages respecting NO_COLOR

## Task Commits

Each task was committed atomically:

1. **Task 1: Create Cargo.toml and CLI type definitions** - `072d874` (feat)
2. **Task 2: Wire main.rs and stub command handlers** - `b6f4ff1` (feat)

## Files Created/Modified
- `Cargo.toml` - Project manifest with all Phase 1 dependencies
- `src/cli.rs` - CLI type definitions: Cli, GlobalArgs, Commands, Language, ModelSize, all arg structs
- `src/main.rs` - Entry point with CLI parsing and command dispatch
- `src/commands/mod.rs` - Module declarations for all command handlers
- `src/commands/design.rs` - Stub handler printing Phase 2 message
- `src/commands/clone.rs` - Stub handler printing Phase 2 message
- `src/commands/generate.rs` - Stub handler printing Phase 2 message
- `src/commands/profiles.rs` - Placeholder handlers for list/show/delete
- `src/commands/model.rs` - Placeholder handlers for download/list/remove
- `src/commands/doctor.rs` - Placeholder handler

## Decisions Made
- Used `owo_colors::Style` builder with `if_supports_color` instead of chained `.yellow().bold()` which caused temporary reference lifetime errors in closures
- Included all planned dependencies (pyo3, serde, directories, etc.) in Cargo.toml from the start even though only clap/owo-colors are used now, for faster incremental compilation in future plans

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Fixed owo-colors chained method lifetime error**
- **Found during:** Task 2 (Wire main.rs and stub command handlers)
- **Issue:** `t.yellow().bold()` inside `if_supports_color` closure caused E0515 (returning reference to temporary)
- **Fix:** Used `Style::new().yellow().bold()` builder then `t.style(style)` which avoids the temporary reference chain
- **Files modified:** src/commands/design.rs, src/commands/clone.rs, src/commands/generate.rs
- **Verification:** `cargo build` succeeds, colored output works
- **Committed in:** b6f4ff1 (Task 2 commit)

---

**Total deviations:** 1 auto-fixed (1 bug)
**Impact on plan:** Minor API usage correction. No scope change.

## Issues Encountered
None beyond the owo-colors lifetime issue documented above.

## Known Stubs

- `src/commands/design.rs` - prints "not yet implemented" (intentional, resolved in Phase 2 per plan)
- `src/commands/clone.rs` - prints "not yet implemented" (intentional, resolved in Phase 2 per plan)
- `src/commands/generate.rs` - prints "not yet implemented" (intentional, resolved in Phase 2 per plan)
- `src/commands/profiles.rs` - placeholder messages (resolved in Plan 01-02 or Phase 2)
- `src/commands/model.rs` - placeholder messages (resolved in Plan 01-02)
- `src/commands/doctor.rs` - placeholder message (resolved in Plan 01-03)

All stubs are intentional per the plan and have clear resolution paths in subsequent plans/phases.

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- Project scaffold complete, ready for Plan 02 (PyO3 Python bridge)
- All CLI types defined and exported for use by subsequent plans
- Command dispatch pattern established for future implementation

## Self-Check: PASSED

All 10 created files verified present. Both task commits (072d874, b6f4ff1) verified in git log.

---
*Phase: 01-foundation-and-python-bridge*
*Completed: 2026-03-27*
