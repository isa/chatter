# Phase 1: Foundation and Python Bridge - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-03-27
**Phase:** 01-foundation-and-python-bridge
**Areas discussed:** Model loading timing, Error presentation, Progress feedback, Environment validation

---

## Model Loading Timing

| Option | Description | Selected |
|--------|-------------|----------|
| Lazy | Only load when a command actually needs inference. --help and profiles list stay instant. | |
| Eager | Always initialize Python + load model on startup. Simpler code path but --help takes seconds. | |
| Two-stage | Always init Python (fast), but defer model loading until inference needed. | |

**User's choice:** Free-text response — wants model management subcommands (download/list/load) for offline usage, with lazy loading + explicit progress bar when model loads. This expanded Phase 1 scope.
**Notes:** User explicitly requested model management commands be included in Phase 1, not deferred.

### Follow-up: --help subcommand visibility

| Option | Description | Selected |
|--------|-------------|----------|
| Show all planned subcommands | Unimplemented ones return "coming soon" | |
| Only show what works | Subcommands appear as phases deliver them | |

**User's choice:** "Update the plan and include this in phase 1" — all subcommands visible from start.

---

## Error Presentation

| Option | Description | Selected |
|--------|-------------|----------|
| Detailed diagnostic | Color-coded error + specific fix command | |
| Minimal + verbose flag | Short error by default, --verbose shows full diagnostic | ✓ |
| Rustc-style | Structured error with error code, description, help suggestion | |

**User's choice:** Minimal + verbose flag

### Follow-up: Color usage

| Option | Description | Selected |
|--------|-------------|----------|
| Yes — color-coded | Red errors, yellow warnings, respects NO_COLOR | ✓ |
| No color | Plain text only | |

**User's choice:** Yes, with NO_COLOR support

---

## Progress Feedback

| Option | Description | Selected |
|--------|-------------|----------|
| Spinner with status message | e.g. "Initializing Python..." then "Loading Qwen3-TTS 1.7B..." | ✓ |
| Progress bar with phases | Multi-step bar showing discrete chunks | |
| Minimal | Just print "Loading model..." then "Ready." | |

**User's choice:** Spinner with status message

### Follow-up: Elapsed time

| Option | Description | Selected |
|--------|-------------|----------|
| Yes — show elapsed time | e.g. "Loading Qwen3-TTS 1.7B... (12s)" | ✓ |
| No — just spinner and message | Keep it clean | |

**User's choice:** Yes, show elapsed time

---

## Environment Validation

| Option | Description | Selected |
|--------|-------------|----------|
| Explicit `chatter doctor` | Full check subcommand. Inline commands just fail with short errors. | ✓ |
| Inline only | No dedicated command. Validation on design/clone/generate. | |
| Both | Doctor command AND inline validation | |

**User's choice:** Explicit `chatter doctor` subcommand

### Follow-up: Doctor scope

| Option | Description | Selected |
|--------|-------------|----------|
| Full system report | Python, qwen-tts, CUDA, GPU, VRAM, PyTorch. Pass/fail per item. | ✓ |
| Essentials only | Python? qwen-tts? GPU? Three checks. | |

**User's choice:** Full system report — but noted "I'm using this on a Mac, MLX should also be an option"

### Follow-up: Compute backend priority

| Option | Description | Selected |
|--------|-------------|----------|
| MLX primary, CUDA secondary | Mac daily driver, CUDA later | |
| Both equally | Runtime detection — MLX on Mac, CUDA on Linux | ✓ |
| MLX only for v1 | CUDA is v2 concern | |

**User's choice:** Both equally — runtime backend detection

---

## Claude's Discretion

- Clap subcommand structure and flag naming
- PyO3 initialization patterns
- Project directory layout
- "Coming soon" message wording

## Deferred Ideas

None — all discussed items were included in Phase 1 scope.
