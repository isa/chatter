# Phase 2: Voice Profiles and Speech Generation - Context

**Gathered:** 2026-03-27
**Status:** Ready for planning

<domain>
## Phase Boundary

Users can create reusable voice profiles (by natural language description or by cloning from audio) and generate speech from inline text using those profiles. This phase delivers the core product value: `chatter design`, `chatter clone`, `chatter generate`, and `chatter profiles list/show`. File-based input (TXT/MD/PDF) is Phase 3.

</domain>

<decisions>
## Implementation Decisions

### Model Size
- **D-01:** Drop `--model-size` flag entirely. Hardcode 1.7B for all operations. Remove 0.6B variant handling from the codebase. Simplifies CLI and model management.

### MLX Model Variants
- **D-02:** When MLX backend is detected (Apple Silicon), use `mlx-community/Qwen3-TTS-12Hz-1.7B-*-bf16` model variants instead of `Qwen/Qwen3-TTS-12Hz-1.7B-*`. Apply to all three model types: VoiceDesign, Base, CustomVoice. Researcher must verify which mlx-community variants exist on HuggingFace.
- **D-03:** CUDA backend continues using `Qwen/Qwen3-TTS-12Hz-1.7B-*` (original PyTorch variants).

### Profile Storage Format
- **D-04:** TOML for profile metadata. Add `toml` crate to Cargo.toml. Human-editable format, idiomatic for Rust.
- **D-05:** One directory per profile: `~/.config/chatter/profiles/{name}/` containing `profile.toml` (metadata) and `sample.mp3` (cached preview audio).
- **D-06:** Profile metadata includes: name, type (designed/cloned), language, description (for designed) or source audio path (for cloned), creation date, model variant used.
- **D-07:** Voice identity stored as raw codes/embeddings from the VoiceDesign model output. Saved to the profile directory for reload at generation time without re-running VoiceDesign.

### Profile Naming
- **D-08:** When `--name` is omitted, auto-generate from the description by slugifying the first few words (e.g., "warm friendly male" -> `warm-friendly-male`). For clone, slugify the source filename.

### Voice Design Flow (Interactive Preview)
- **D-09:** After generating the voice, automatically play the cached sample audio in the terminal with a message like "Preview of your custom voice...". Use `afplay` on macOS, `aplay`/`paplay` on Linux (shell out, no audio playback crates).
- **D-10:** If the user is not happy with the result, they can tweak the description and regenerate. Interactive loop: generate -> preview -> accept or retry with modified description.
- **D-11:** Design uses VoiceDesign 1.7B model only (no 0.6B variant exists). `--model-size` flag is removed entirely per D-01.

### Clone Flow
- **D-12:** No interactive preview loop for clone — it's deterministic (same input = same output). Save the profile and print the sample MP3 path.
- **D-13:** Strict input validation for clone: check file exists, validate format (MP3/WAV), warn if duration is too short or too long, check sample rate.

### Cached Sample Audio
- **D-14:** Fixed preview sentence generated at profile creation time: "Hello, this is a preview of your voice profile." (~3-5 seconds of audio). Same sentence for both designed and cloned profiles.

### Speech Generation Output
- **D-15:** Default output path uses profile name + timestamp: `./warm-friendly-male-20260327-143022.mp3`. Written to current working directory.
- **D-16:** If output file already exists, overwrite it but print a warning message noting the file was replaced.
- **D-17:** Progress bar with percentage during speech synthesis if qwen-tts provides chunk-level progress callbacks. Fallback to spinner + elapsed time (Phase 1 style) if no granular progress is available.
- **D-18:** Add `--play` flag to `generate` command for optional audio playback after generation. Uses same `afplay`/`aplay` shell-out as design preview.

### Profile Listing Display
- **D-19:** `chatter profiles list` shows a simple table: Name, Type (designed/cloned), Language, Created date. One line per profile. Human-readable only, no `--json` flag for v1.
- **D-20:** `chatter profiles show {name}` shows full detail dump: all metadata fields, sample audio path, description/source, file sizes.

### Claude's Discretion
- Exact TOML schema field names and structure
- How voice embeddings/codes are serialized to disk (binary vs base64 in TOML vs separate file)
- Specific slugification algorithm and collision handling (e.g., `warm-friendly-male-2`)
- Audio validation thresholds for clone (min/max duration, accepted sample rates)
- Progress bar vs spinner decision based on what qwen-tts API actually exposes
- Table formatting library choice or manual formatting for `profiles list`

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Project Definition
- `.planning/PROJECT.md` -- Core value, constraints, key decisions
- `.planning/REQUIREMENTS.md` -- PROF-01 through PROF-06, GEN-01, GEN-05, GEN-06, UX-02, UX-03 are Phase 2 requirements
- `.planning/ROADMAP.md` -- Phase 2 success criteria and dependency chain

### Technology Stack
- `CLAUDE.md` &sect;Technology Stack -- Full recommended stack with versions, PyO3 integration strategy, audio pipeline

### Phase 1 Context
- `.planning/phases/01-foundation-and-python-bridge/01-CONTEXT.md` -- Prior decisions on model loading, error handling, progress feedback, compute backends

### External References
- [Qwen3-TTS GitHub](https://github.com/QwenLM/Qwen3-TTS) -- Model documentation, VoiceDesign API, CustomVoice API
- [qwen-tts PyPI](https://pypi.org/project/qwen-tts/) -- Python package API for voice design, cloning, generation
- [mlx-community/Qwen3-TTS-12Hz-1.7B-CustomVoice-bf16](https://huggingface.co/mlx-community/Qwen3-TTS-12Hz-1.7B-CustomVoice-bf16) -- MLX-optimized model variant (verify all three variants exist)
- [PyO3 User Guide](https://pyo3.rs/) -- Embedding patterns for Python callback integration

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `src/ui.rs`: `create_spinner()` for progress feedback during model loading/inference
- `src/ui.rs`: `print_error()` with verbose/minimal modes and NO_COLOR support
- `src/bridge/runtime.rs`: `detect_backend()` / `ComputeBackend` enum -- drives MLX vs CUDA model variant selection (D-02/D-03)
- `src/bridge/model.rs`: `model_variants()`, `download_model()`, `list_cached_models()` -- needs updating for 1.7B-only and MLX variants
- `src/bridge/venv.rs`: Python venv management -- auto-setup on first run
- `src/cli.rs`: `DesignArgs`, `CloneArgs`, `GenerateArgs`, `ProfilesCommands` already defined

### Established Patterns
- PyO3 `Python::attach(|py| { ... })` for all Python interop
- `BridgeError` enum with `thiserror` for typed errors from Python bridge
- `import_hf_hub()` helper pattern for Python module imports with friendly errors
- owo-colors `if_supports_color` for NO_COLOR-compliant output

### Integration Points
- `src/commands/design.rs`, `clone.rs`, `generate.rs`: Currently stubs, need full implementation
- `src/commands/profiles.rs`: Currently returns "not found" stubs, needs profile storage integration
- `src/bridge/model.rs`: `model_variants()` needs refactoring to remove 0.6B and add MLX variant selection
- `src/cli.rs`: `ModelSize` enum and `--model-size` flag need removal (D-01)
- `Cargo.toml`: Needs `toml`, `hound`, `mp3lame-encoder` crates added

</code_context>

<specifics>
## Specific Ideas

- User is on Mac -- MLX is the primary runtime. MLX model variants from `mlx-community` should be the default path on Apple Silicon.
- Interactive design loop is a key UX differentiator: generate voice, hear it, tweak description, regenerate until satisfied.
- Audio playback via system commands (`afplay` on Mac) keeps the dependency footprint minimal.
- Profile directories make it easy for users to inspect/backup/share profiles manually.

</specifics>

<deferred>
## Deferred Ideas

None -- discussion stayed within phase scope.

</deferred>

---

*Phase: 02-voice-profiles-and-speech-generation*
*Context gathered: 2026-03-27*
