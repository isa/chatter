# Phase 07: Model Management and Diagnostics - Context

**Gathered:** 2026-03-29
**Status:** Ready for planning

<domain>
## Phase Boundary

Extend `chatter model` and `chatter doctor` commands to fully support ChatterBox models. Users can download, list, and remove ChatterBox model variants with disk space awareness, and doctor validates ChatterBox installation alongside Qwen3-TTS.

</domain>

<decisions>
## Implementation Decisions

### Model Download UX
- **D-01:** `chatter model download --engine chatterbox` downloads all 3 variants (Original, Turbo, Multilingual) together. One command gets everything.
- **D-02:** Show estimated total size and available disk space before starting download. Warn if less than 5GB would remain after download. Auto-proceed (no interactive confirmation required) — just inform the user.

### Doctor ChatterBox Checks
- **D-03:** `chatter doctor` always shows both engines (Qwen and ChatterBox). If ChatterBox isn't installed, show it as "not installed" (informational, not a failure). User always sees the full picture.
- **D-04:** ChatterBox-specific checks: (1) chatterbox-tts installed + version, (2) MPS/CUDA available for ChatterBox, (3) MLX community models available if on Apple Silicon, (4) sufficient disk space for ChatterBox models (~15-20GB).

### Model Variant Listing
- **D-05:** `chatter model list` groups models by engine (Qwen section, ChatterBox section). Shows variant name (Original, Turbo, Multilingual) alongside repo ID. Shows download status and size.

### Doctor --fix Behavior
- **D-06:** `chatter doctor --fix` installs both Qwen and ChatterBox deps/models if missing. One command gets everything working.

### Claude's Discretion
- Exact format of disk space warning messages
- Doctor output formatting details (spacing, grouping)
- How to detect MLX community model availability for ChatterBox
- Model size estimates for display (can be hardcoded or queried from HF cache)

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Existing Model Management
- `src/commands/model.rs` — Current model command handler with engine dispatch (download/list/remove for both engines)
- `src/bridge/model.rs` — `download_model_chatterbox()`, `chatterbox_model_variants()`, `list_cached_models()`, `remove_chatterbox_models()`

### Existing Doctor
- `src/commands/doctor.rs` — Doctor command renderer, `--fix` flag handler, output formatting
- `src/bridge/doctor.rs` — `SystemInfo` struct, `get_system_info()`, disk/cache info gathering

### Phase 06 Implementation (already in codebase)
- `src/bridge/venv.rs` — `install_chatterbox_deps()`, `is_chatterbox_installed()` — already implemented
- `src/bridge/error.rs` — `ChatterBoxNotInstalled` error variant
- `requirements/chatterbox.txt` — Curated dependency list

### Research
- `.planning/research/SUMMARY.md` — ChatterBox model sizes, HuggingFace repo IDs

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `download_model_chatterbox()` — already downloads all ChatterBox variants from HF. Needs disk space pre-check added.
- `list_cached_models()` — already recognizes ChatterBox repos. Needs variant label enrichment.
- `is_chatterbox_installed()` — ready-made check for doctor.
- `get_system_info()` — needs extension for ChatterBox package version.
- `ui::doctor_pass/fail/warn` — existing doctor output helpers.

### Established Patterns
- Doctor checks follow: check condition -> `ui::doctor_pass/fail/warn` pattern
- Model download uses HuggingFace `snapshot_download` via Python bridge
- Model listing reads from HF cache directory structure
- `--fix` flag triggers auto-download of missing models

### Integration Points
- `SystemInfo` struct needs new fields for ChatterBox package version and ChatterBox-specific model status
- `commands/doctor.rs` needs new section for ChatterBox checks
- `commands/model.rs` download handler needs disk space pre-check before downloading
- `commands/model.rs` list handler needs engine-grouped output format

</code_context>

<specifics>
## Specific Ideas

- Doctor should be engine-aware but always show both — not filtered by `--engine`
- Disk space warning is informational, not blocking — auto-proceed after showing info
- Phase 06 already did most of the model download/list/remove plumbing — this phase is about UX polish and doctor extension

</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope.

</deferred>

---

*Phase: 07-model-management-and-diagnostics*
*Context gathered: 2026-03-29*
