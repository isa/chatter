# Phase 08: ChatterBox Controls - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-03-29
**Phase:** 08-chatterbox-controls
**Areas discussed:** Exaggeration & CFG flags, Paralinguistic tag validation, Engine-specific flag gating

---

## Exaggeration & CFG Flags

### Default values

| Option | Description | Selected |
|--------|-------------|----------|
| exaggeration=0.5, cfg=0.5 | Match current hardcoded values. | ✓ |
| No defaults, always required | User must specify. | |
| You decide | Claude picks. | |

**User's choice:** exaggeration=0.5, cfg=0.5

### Scope (clone vs generate)

| Option | Description | Selected |
|--------|-------------|----------|
| Generate only | Clone preview uses defaults. | ✓ |
| Both clone and generate | User controls during clone too. | |
| You decide | Claude picks. | |

**User's choice:** Generate only

---

## Paralinguistic Tag Validation

### Validation location

| Option | Description | Selected |
|--------|-------------|----------|
| Rust-side, before bridge call | Early validation, clear CLI error. | ✓ |
| Python-side, in chatterbox.py | Simpler but Python exceptions. | |
| You decide | Claude picks. | |

**User's choice:** Rust-side, before bridge call

### Accepted tags

| Option | Description | Selected |
|--------|-------------|----------|
| Official set from docs | [laugh], [chuckle], [cough], [sigh], [gasp], [groan], [yawn], [cry] only. | ✓ |
| Permissive: any [tag] format | Accept anything in brackets. | |
| You decide | Claude picks. | |

**User's choice:** Official set from docs

---

## Engine-Specific Flag Gating

### Wrong engine behavior

| Option | Description | Selected |
|--------|-------------|----------|
| Error with clear message | Exit with error for wrong engine. | ✓ |
| Warning, then ignore | Print warning but continue. | |
| Silently ignore | No feedback. | |

**User's choice:** Error with clear message

### Wrong variant behavior

| Option | Description | Selected |
|--------|-------------|----------|
| Error for wrong engine, warn for wrong variant | Proportional strictness. | ✓ |
| Error for both | Strict everywhere. | |
| You decide | Claude picks. | |

**User's choice:** Error for wrong engine, warn for wrong variant

---

## Claude's Discretion

- Error/warning message wording
- PyO3 bridge parameter passing
- Tag validation function structure
- Optional --tags help flag

## Deferred Ideas

None.
