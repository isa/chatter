# Phase 06: ChatterBox Inference - Context

**Gathered:** 2026-03-29
**Status:** Ready for planning

<domain>
## Phase Boundary

Implement ChatterBox voice cloning and speech generation as a fully working engine module. Users can clone voices from audio samples and generate speech using ChatterBox models via `--engine chatterbox`. Covers Apple Silicon (MLX-first with MPS fallback) and CUDA. Includes dependency installation strategy and memory management for safe engine switching on 16GB Macs.

</domain>

<decisions>
## Implementation Decisions

### Backend Strategy
- **D-01:** MLX-first with MPS/PyTorch fallback on Apple Silicon. Try `mlx-audio` community models (`mlx-community/chatterbox-*`) first. If validation fails, silently fall back to MPS with a one-time info message ("Using MPS backend for ChatterBox").
- **D-02:** Both Apple Silicon and CUDA must be supported in this phase — not Apple Silicon only.
- **D-03:** Backend detection is per-engine (already established in Phase 04). ChatterBox module owns its own `detect_backend()`.

### Voice Cloning Model
- **D-04:** ChatterBox profiles store reference audio only (`ref_audio.wav`). No pre-computed voice prompts — ChatterBox passes reference audio directly at inference time.
- **D-05:** The clone command uses the same preview-listen-retry loop as Qwen for consistent UX across engines.
- **D-06:** Default model variant with override. Use Original for English, Multilingual for non-English as sensible defaults. User can override with `--variant turbo|original|multilingual`. Profile stores which variant was used.

### Dependency Installation
- **D-07:** Install `chatterbox-tts` with `--no-deps` plus a curated requirements list checked into the repo. Avoids gradio bloat (~200MB) and controls exact versions.
- **D-08:** Explicit install, not lazy. User must run `chatter model download --engine chatterbox` before first use. If ChatterBox deps are not installed and user tries `--engine chatterbox`, show a clear error directing them to the download command.

### Memory Management
- **D-09:** Full cleanup on engine switch: `unload_all_models()` + `del` references + `gc.collect()` + `torch.mps.empty_cache()` (Apple Silicon) or `torch.cuda.empty_cache()` (CUDA). Maximum memory reclamation to prevent OOM.
- **D-10:** Engine switching is automatic on `--engine` flag. If user runs `--engine chatterbox` and qwen models are loaded, automatically unload qwen first. The `--engine` flag is the explicit intent signal — no confirmation prompt needed.

### Claude's Discretion
- MLX validation approach (how exactly to test community models at phase start)
- Curated requirements list contents for `chatterbox-tts --no-deps`
- Error message wording for missing ChatterBox deps
- ChatterBox `generate_speech()` and `voice_clone_from_audio()` internal implementation details

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Existing Engine Implementation (reference pattern)
- `chatter_bridge/engines/qwen.py` — Reference implementation for the engine module pattern (backend detection, model loading, inference, unloading)
- `chatter_bridge/engines/chatterbox.py` — Stub with all function signatures to implement
- `chatter_bridge/__init__.py` — Dispatcher that routes calls to active engine module

### Rust Bridge Layer
- `src/bridge/inference.rs` — PyO3 bridge calls (set_engine, generate_speech, voice_clone_from_audio, etc.)
- `src/bridge/venv.rs` — Managed venv setup where chatterbox-tts needs to be installed
- `src/bridge/error.rs` — BridgeError types

### CLI Commands
- `src/commands/clone.rs` — Existing clone workflow with preview loop, profile save, WAV/MP3 encoding
- `src/commands/generate.rs` — Existing generate workflow

### Research
- `.planning/research/SUMMARY.md` — ChatterBox API details, model variants, dependency conflicts, MLX community models, memory estimates

### External Documentation
- [Resemble AI ChatterBox GitHub](https://github.com/resemble-ai/chatterbox) — Official API, pyproject.toml
- [mlx-community ChatterBox models](https://huggingface.co/mlx-community/chatterbox-fp16) — MLX model availability
- [Jimmi42/chatterbox-tts-apple-silicon-code](https://huggingface.co/Jimmi42/chatterbox-tts-apple-silicon-code) — Apple Silicon MPS adaptation

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `engines/qwen.py` — Full reference for engine module structure: `_backend_cache`, `_models` dict, `detect_backend()`, `load_*_model()`, `generate_speech()`, `unload_all_models()`
- `bridge/inference.rs` — All PyO3 bridge functions already dispatch through the active engine. No Rust changes needed for core inference.
- `commands/clone.rs` — Preview loop, WAV encoding, profile metadata save. Engine-aware via `global.engine.as_str()`.
- `audio` module — `samples_f32_to_i16()` and `encode_wav_to_mp3()` work with any 24kHz float output.

### Established Patterns
- Engine modules must expose: `detect_backend()`, `set_mlx_quantization()`, `load_*_model()`, `voice_design()`, `create_clone_prompt()`, `save_clone_prompt()`, `load_clone_prompt()`, `generate_speech()`, `voice_clone_from_audio()`, `is_model_loaded()`, `ensure_model()`, `unload_all_models()`
- ChatterBox has no `voice_design()` — this should raise `NotImplementedError` (already stubbed)
- ChatterBox has no pre-computed prompts — `create_clone_prompt()`, `save_clone_prompt()`, `load_clone_prompt()` should be no-ops or raise appropriately
- Backend detection is per-engine module (not global)

### Integration Points
- `venv.rs` — Must embed ChatterBox curated requirements and install on `chatter model download --engine chatterbox`
- `commands/model.rs` — Must support `--engine chatterbox` for the download trigger
- `bridge/__init__.py` `set_engine()` — Already routes to engine modules; ChatterBox module just needs real implementations

</code_context>

<specifics>
## Specific Ideas

- User explicitly stated: no lazy/automatic dependency installation. The `chatter model download --engine chatterbox` command is the gate. Show a warning/error if deps missing.
- Both CUDA and Apple Silicon must work — not a phased hardware rollout.
- Preview loop must feel the same across engines for consistent UX.

</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope.

</deferred>

---

*Phase: 06-chatterbox-inference*
*Context gathered: 2026-03-29*
