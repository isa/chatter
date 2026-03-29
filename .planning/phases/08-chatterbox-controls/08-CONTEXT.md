# Phase 08: ChatterBox Controls - Context

**Gathered:** 2026-03-29
**Status:** Ready for planning

<domain>
## Phase Boundary

Expose ChatterBox-specific generation parameters (`--exaggeration`, `--cfg`) for the Original variant and add paralinguistic tag validation (`[laugh]`, `[sigh]`, etc.) for the Turbo variant. These are engine-specific controls not available in Qwen3-TTS.

</domain>

<decisions>
## Implementation Decisions

### Exaggeration & CFG Flags (FT-01)
- **D-01:** Default values: `exaggeration=0.5`, `cfg=0.5` — matches current hardcoded values in `chatterbox.py`. Users get identical behavior unless they explicitly override.
- **D-02:** These flags apply to `generate` command only, not `clone` preview. Clone preview uses defaults for consistency.
- **D-03:** Flags are `--exaggeration` (f64, 0.0-1.0 range) and `--cfg` (f64) on `GenerateArgs` in `cli.rs`.

### Paralinguistic Tag Validation (FT-02)
- **D-04:** Validation happens in Rust, before the Python bridge call. Catches errors early with a clear CLI error message — no Python traceback reaches the user.
- **D-05:** Official tag set only: `[laugh]`, `[chuckle]`, `[cough]`, `[sigh]`, `[gasp]`, `[groan]`, `[yawn]`, `[cry]`. Reject any other `[tag]` format.
- **D-06:** Validation only triggers when engine is ChatterBox and variant is Turbo. Other variants/engines ignore tags (they just appear as literal text).

### Engine-Specific Flag Gating
- **D-07:** Using `--exaggeration` or `--cfg` with `--engine qwen` produces an error: `"--exaggeration is only available with --engine chatterbox"`. Same pattern as design being Qwen-only.
- **D-08:** Using `--exaggeration` or `--cfg` with ChatterBox Turbo or Multilingual produces a warning (not error): `"--exaggeration has no effect with ChatterBox Turbo variant"`. Flag is accepted but ignored.

### Claude's Discretion
- Exact error/warning message wording
- How to pass exaggeration/cfg through the PyO3 bridge to the Python engine
- Internal structure of the tag validation function
- Whether to add a `--tags` help flag that lists available paralinguistic tags

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### ChatterBox Engine (modify)
- `chatter_bridge/engines/chatterbox.py` — `_generate_with_model()` has hardcoded `exaggeration=0.5, cfg_weight=0.5` that needs to accept parameters
- `chatter_bridge/__init__.py` — Dispatcher `generate_speech()` signature may need new params

### Rust CLI & Bridge (modify)
- `src/cli.rs` — `GenerateArgs` needs `--exaggeration` and `--cfg` flags
- `src/commands/generate.rs` — Needs to validate flags, gate by engine/variant, pass to bridge
- `src/bridge/inference.rs` — `generate_speech()` needs to pass new params through PyO3

### Research
- `.planning/research/SUMMARY.md` — ChatterBox API details, paralinguistic tags reference
- [Paralinguistic Tags Guide](https://www.mintlify.com/yocxy2/chatterboxyocxy/guides/paralinguistic-tags) — Full tag list, syntax rules

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `chatterbox.py::_generate_with_model()` — Already has variant branching for Original/Turbo/Multilingual. Just needs to accept exaggeration/cfg as parameters instead of hardcoding.
- `cli.rs::GenerateArgs` — Already has `--cb-variant` flag. New flags follow same pattern.
- Engine gating pattern established: `design` command errors for ChatterBox, clone/generate check engine.

### Established Patterns
- Engine-specific validation in command handlers (early exit with `anyhow::bail!`)
- PyO3 bridge passes named parameters via `call_method1` tuples
- Python `generate_speech()` accepts extra kwargs and routes variant-aware

### Integration Points
- `generate_speech()` in `chatter_bridge/__init__.py` needs new parameters (exaggeration, cfg_weight)
- `generate_speech()` in `chatter_bridge/engines/chatterbox.py` passes to `_generate_with_model()`
- `bridge::inference::generate_speech()` in Rust passes through PyO3

</code_context>

<specifics>
## Specific Ideas

- Tag validation should be a standalone function in a new module or in generate.rs — reusable for any future text preprocessing
- The official ChatterBox paralinguistic tags from documentation: `[laugh]`, `[chuckle]`, `[cough]`, `[sigh]`, `[gasp]`, `[groan]`, `[yawn]`, `[cry]`

</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope.

</deferred>

---

*Phase: 08-chatterbox-controls*
*Context gathered: 2026-03-29*
