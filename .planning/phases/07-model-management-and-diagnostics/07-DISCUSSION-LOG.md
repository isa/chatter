# Phase 07: Model Management and Diagnostics - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-03-29
**Phase:** 07-model-management-and-diagnostics
**Areas discussed:** Model download UX, Doctor ChatterBox checks, Model variant listing, Doctor --fix behavior

---

## Model Download UX

### Variant download scope

| Option | Description | Selected |
|--------|-------------|----------|
| Download all variants | Download Original, Turbo, Multilingual together. Show total size upfront. | ✓ |
| Interactive variant picker | Let user select which variant(s). | |
| Single default, opt-in others | Download Original by default. | |

**User's choice:** Download all variants
**Notes:** None.

### Disk space warnings (MDL-03)

| Option | Description | Selected |
|--------|-------------|----------|
| Show sizes + auto-proceed | Display size and free space, warn if tight. Don't require confirmation. | ✓ |
| Interactive confirmation | Require user to confirm before downloading. | |
| You decide | Claude picks. | |

**User's choice:** Show sizes + auto-proceed
**Notes:** None.

---

## Doctor ChatterBox Checks

### Doctor engine scope

| Option | Description | Selected |
|--------|-------------|----------|
| Always show both | Show Qwen and ChatterBox status every time. CB "not installed" is informational. | ✓ |
| Only installed engines | Only show ChatterBox if installed. | |
| Engine-filtered via --engine | Show only specified engine. | |

**User's choice:** Always show both
**Notes:** None.

### ChatterBox check set

| Option | Description | Selected |
|--------|-------------|----------|
| Package + hardware + disk | Check package version, MPS/CUDA, MLX community models, disk space. | ✓ |
| Minimal: package only | Just package install status. | |
| You decide | Claude picks. | |

**User's choice:** Package + hardware + disk
**Notes:** None.

---

## Model Variant Listing

### List display format

| Option | Description | Selected |
|--------|-------------|----------|
| Engine-grouped with variant labels | Group by engine, show variant name, download status, size. | ✓ |
| Flat list with engine column | Single table with Engine column. | |
| You decide | Claude picks. | |

**User's choice:** Engine-grouped with variant labels
**Notes:** None.

---

## Doctor --fix Behavior

### Fix scope

| Option | Description | Selected |
|--------|-------------|----------|
| Fix both engines | --fix installs both Qwen and ChatterBox if missing. | ✓ |
| Fix active engine only | --fix only fixes --engine specified engine. | |
| You decide | Claude picks. | |

**User's choice:** Fix both engines
**Notes:** None.

---

## Claude's Discretion

- Disk space warning message format
- Doctor output formatting details
- MLX community model detection approach
- Model size display (hardcoded vs queried)

## Deferred Ideas

None — discussion stayed within phase scope.
