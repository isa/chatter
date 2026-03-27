# Project Research Summary

**Project:** Chatter (Rust TTS CLI with PyO3 + Qwen3-TTS)
**Domain:** Local TTS CLI with embedded Python ML inference
**Researched:** 2026-03-27
**Confidence:** MEDIUM-HIGH

## Executive Summary

Chatter is a Rust CLI tool that embeds a Python runtime (via PyO3) to run Qwen3-TTS model inference locally on GPU. The product's differentiation is threefold: voice design from natural language descriptions, voice cloning from audio samples, and persistent named voice profiles. No existing TTS CLI tool combines these three capabilities. The recommended approach is a layered Rust architecture where all Python interaction is isolated behind a single engine boundary, with pure Rust handling CLI parsing, profile storage, and audio encoding (WAV-to-MP3).

The core technical challenge is the PyO3-Python bridge. Embedding a Python interpreter in Rust for ML inference is well-documented but has sharp edges: virtual environment detection failures, GIL deadlocks with progress callbacks, and GPU memory leaks across inference calls. The qwen-tts package itself (v0.1.1) has dependency conflicts that constrain the viable Python version to 3.10.x. These issues must be resolved in the foundation phase before any feature work begins, as they are binary blockers -- if the Python environment does not work, nothing else matters.

The stack is mature and well-chosen: clap for CLI, indicatif for progress, serde/TOML for profiles, mp3lame-encoder for audio, and PyO3 0.28 for the Python bridge. The riskiest areas are the Python dependency chain (qwen-tts pins conflicting versions) and the voice design reuse pattern (VoiceDesign output is non-deterministic and requires a two-step clone-prompt caching workflow to produce consistent voices). Both of these are solvable with upfront design but will cause significant rework if discovered late.

## Key Findings

### Recommended Stack

The stack is entirely local -- no network dependencies, no async runtime, no database. Rust handles everything except ML inference, which stays in Python because rewriting PyTorch + transformers in Rust is impractical.

**Core technologies:**
- **Rust (2024 edition, 1.85+)**: Systems performance, strong CLI ecosystem (clap, indicatif, owo-colors)
- **PyO3 0.28.2**: Only mature Rust-Python interop crate; embeds CPython in-process for low-latency model calls
- **clap 4.5+**: De facto Rust CLI framework with derive macros
- **mp3lame-encoder 0.2.2**: Static LAME bindings for WAV-to-MP3 without external dependencies
- **hound 3.5.1**: Standard Rust WAV library for reading model output
- **pdf-extract 0.10 + pulldown-cmark 0.13**: File parsing for PDF and Markdown input
- **indicatif 0.18 + owo-colors 4.x**: Progress bars and terminal coloring
- **serde + TOML**: Profile metadata serialization; directory-per-profile with sidecar audio
- **directories 6.0**: Cross-platform XDG path resolution for config/data storage
- **Python 3.10.x + qwen-tts 0.1.1**: Constrained by dependency conflicts in qwen-tts package

**What NOT to use:** No async runtime (tokio), no HTTP clients, no database, no TUI framework, no audio playback. This is a synchronous, local, generate-to-file CLI tool.

### Expected Features

**Must have (table stakes):**
- Text-to-speech from string, stdin, and TXT/Markdown files
- Voice selection via named profiles (`--profile <name>`)
- WAV and MP3 output formats
- Progress feedback during model loading and inference
- Language selection with auto-detect default
- Model size selection (0.6B / 1.7B)
- Help, discoverability (`--list-profiles`, `--list-languages`)
- Clear error messages with setup guidance

**Should have (differentiators -- these ARE the product):**
- Voice design from natural language description (unique to Chatter; no competitor has this)
- Voice cloning from audio sample (Coqui has this, but most CLI tools do not)
- Persistent named voice profiles with metadata and cached preview audio
- Smart text preprocessing (strip markdown formatting before synthesis)

**Defer (v2+):**
- Streaming audio playback (massive complexity, different product)
- PDF input (add in v1.x after core pipeline is solid)
- Long document chunking with per-chunk progress (v1.x)
- Voice profile export/import, batch processing, subtitle generation
- Cloud/API integration, GUI/TUI, SSML support

### Architecture Approach

The architecture is a four-layer stack: CLI (thin command dispatch) -> Orchestration (profile store, progress, audio post-processing) -> Python Bridge (single InferenceEngine module, all PyO3 isolated here) -> Python Runtime (qwen-tts, torch, soundfile). The critical boundary is the engine layer: no PyO3 types leak above it, all Python exceptions are caught and converted to typed Rust errors at this boundary, and the model reference is cached as `Py<PyAny>` for the process lifetime.

**Major components:**
1. **CLI Layer** (cli/) -- Thin clap-derived subcommands: design, clone, generate, profiles. No business logic.
2. **InferenceEngine** (engine/) -- ALL PyO3 interaction isolated here. Manages Python interpreter lifecycle, GIL acquisition, model loading/caching, progress callback relay. Exposes pure Rust API.
3. **ProfileStore** (profile/) -- Pure Rust. Directory-per-profile with TOML metadata + sidecar audio files. CRUD operations on `~/.config/chatter/profiles/`.
4. **AudioPostProcessor** (audio/) -- Pure Rust MP3 encoding via mp3lame-encoder. Decoupled from inference for independent testing.
5. **Error types** (error.rs) -- Unified error enum mapping Python exceptions, IO errors, and domain errors.

### Critical Pitfalls

1. **qwen-tts dependency conflicts** -- The package pins conflicting transformers versions and constrains Python to 3.10.x. Resolve before writing any Rust code by testing a clean venv install. Pin known-working versions in a requirements.txt.

2. **PyO3 virtual environment detection** -- Embedded Python may not find venv packages at runtime. Must explicitly configure `sys.path` at startup to include the correct site-packages. Consider a `chatter setup` command that validates the environment.

3. **Voice design reuse pattern** -- VoiceDesign is non-deterministic. The same description produces different voices each run. Profiles must store the generated audio sample AND a pre-computed clone prompt (via `create_voice_clone_prompt`), not just the text description. This two-step caching is essential for consistent voice reproduction.

4. **GPU memory leaks across inference calls** -- PyTorch CUDA tensors held by PyO3 references prevent garbage collection. Must use `torch.inference_mode()`, explicitly call `torch.cuda.empty_cache()` after each inference, and scope GIL acquisitions tightly.

5. **Long text causes speed drift and infinite loops** -- The model's speaking rate accelerates on text over ~100 characters. Very long text can hang indefinitely. Must implement paragraph-aware text chunking with generation timeouts before processing any documents.

6. **First-run model download with no feedback** -- 3-7 GB download happens silently inside Python. Must detect missing models, show download progress, and handle interrupted downloads gracefully.

## Implications for Roadmap

Based on research, suggested phase structure:

### Phase 1: Foundation and Python Bridge

**Rationale:** The PyO3-Python bridge is the highest-risk component. If embedded Python cannot reliably find and load qwen-tts, nothing else works. Dependency conflicts and venv detection are binary blockers that must be resolved first.
**Delivers:** Working Rust binary that initializes Python, resolves dependencies, loads a Qwen3-TTS model, and generates a single audio sample from hardcoded text. Error types and project structure established.
**Addresses:** Error handling framework, Python environment validation, model download UX, GPU detection
**Avoids:** Pitfall 1 (dependency hell), Pitfall 2 (venv detection), Pitfall 7 (silent model download)
**Stack:** PyO3 0.28, anyhow, thiserror, directories, indicatif (for download progress)

### Phase 2: Voice Profiles and Core TTS

**Rationale:** Voice profiles are the product's differentiation and a prerequisite for the generate command. The profile storage schema must account for the voice design reuse pattern (clone-prompt caching) from day one. Core TTS (design, clone, generate) depends on profiles existing.
**Delivers:** `chatter design`, `chatter clone`, `chatter generate` commands. Profile CRUD with TOML metadata and sidecar audio. WAV and MP3 output.
**Addresses:** Voice design, voice cloning, profile persistence, audio encoding, progress bars during inference
**Avoids:** Pitfall 5 (voice design not reusable), Pitfall 3 (GIL deadlocks with progress), Pitfall 4 (GPU memory leaks)
**Stack:** clap, serde, toml, hound, mp3lame-encoder, indicatif, owo-colors, console

### Phase 3: File Input and Text Processing

**Rationale:** File input (TXT, Markdown, PDF) requires text preprocessing and chunking, which depend on a working TTS pipeline. Long text chunking is critical for avoiding model hangs and speed drift.
**Delivers:** `chatter generate --file <path>` for TXT, Markdown, and PDF. Paragraph-aware text splitting. Per-chunk progress reporting.
**Addresses:** File input (TXT/MD/PDF), smart text preprocessing, long document chunking
**Avoids:** Pitfall 6 (long text drift/hangs)
**Stack:** pulldown-cmark, pdf-extract

### Phase 4: Polish and Robustness

**Rationale:** After core features work, harden the user experience. Cross-platform Python detection, profile management ergonomics, help text quality, edge case handling.
**Delivers:** `chatter profiles list/info/delete/preview`, improved error messages, stdin pipe support, auto language detection, `chatter download` command for explicit model pre-download.
**Addresses:** Profile management, stdin piping, auto language detection, discoverability
**Avoids:** UX pitfalls (unclear errors, no preview before save)

### Phase Ordering Rationale

- **Foundation first** because the entire product depends on PyO3 + qwen-tts working. The dependency conflicts and venv issues are blockers that have no workaround.
- **Profiles before generate** because speech generation requires a voice profile to exist. The profile schema must be designed with clone-prompt caching before any voice design code is written.
- **File input last among core features** because it is an input preprocessing concern that layers on top of working TTS. Text chunking is complex and the penalties for getting it wrong (model hangs) are severe, so it benefits from a proven pipeline underneath.
- **Polish after features** because robustness improvements are iterative refinements, not architectural decisions.

### Research Flags

Phases likely needing deeper research during planning:
- **Phase 1:** The qwen-tts dependency resolution needs hands-on validation in a clean venv. PyO3 venv detection needs prototype testing. This phase cannot be planned from docs alone.
- **Phase 2:** The voice design reuse pattern (two-step clone-prompt caching) needs validation against the actual qwen-tts API. Progress callback availability in qwen-tts/transformers generate() needs investigation.

Phases with standard patterns (skip research-phase):
- **Phase 3:** Text chunking, PDF extraction, and Markdown stripping are well-documented problems with established Rust crate solutions.
- **Phase 4:** Profile management CRUD and CLI polish are standard patterns.

## Confidence Assessment

| Area | Confidence | Notes |
|------|------------|-------|
| Stack | HIGH | All crates verified on crates.io/docs.rs with recent release dates. PyO3 0.28 is well-documented. |
| Features | MEDIUM-HIGH | Competitor analysis is thorough. Feature prioritization is sound. PDF deferral is pragmatic. |
| Architecture | MEDIUM-HIGH | Layered architecture with isolated PyO3 boundary is a proven pattern. Build order is dependency-aware. |
| Pitfalls | HIGH | Sourced from PyO3 GitHub issues, Qwen3-TTS issues, and PyTorch forums. Specific issue numbers cited. |

**Overall confidence:** MEDIUM-HIGH

### Gaps to Address

- **qwen-tts API stability:** The package is v0.1.1 with known dependency conflicts. The API may change. Pin versions aggressively and monitor upstream.
- **Progress callback feasibility:** It is unclear whether qwen-tts or transformers expose a generation progress callback. If not, the alternative is monkey-patching or using a spinner instead of a progress bar. Needs hands-on validation in Phase 2.
- **Python version constraint:** Research suggests Python 3.10.x is the only viable version due to dependency conflicts. This needs validation -- if the constraint is real, it significantly limits user compatibility. The STACK.md recommendation of Python 3.12 conflicts with the PITFALLS.md finding of 3.10.x. This must be resolved in Phase 1.
- **Flash Attention availability:** Optional but provides 30-40% speedup. Detection and graceful fallback needs implementation but is low risk.
- **0.6B model quality:** The smaller model has significant quality issues on long text (106 long pauses vs 2 for 1.7B). May need to warn users more aggressively or default to 1.7B only.

## Sources

### Primary (HIGH confidence)
- [PyO3 0.28.2 docs](https://docs.rs/crate/pyo3/latest) -- API, embedding patterns, GIL management
- [PyO3 User Guide](https://pyo3.rs/) -- auto-initialize, with_gil, error handling
- [Qwen3-TTS GitHub](https://github.com/QwenLM/Qwen3-TTS) -- model API, known issues
- [qwen-tts PyPI](https://pypi.org/project/qwen-tts/) -- package version, dependencies
- [clap](https://crates.io/crates/clap), [indicatif](https://docs.rs/crate/indicatif/latest), [hound](https://docs.rs/crate/hound/latest) -- verified crate versions

### Secondary (MEDIUM confidence)
- [PyO3 GitHub discussions](https://github.com/PyO3/pyo3/discussions/) -- GIL deadlocks, venv detection, memory leaks
- [Qwen3-TTS GitHub issues #237, #145, #239](https://github.com/QwenLM/Qwen3-TTS/issues) -- dependency conflicts, speed drift
- [PyTorch GPU memory forums](https://discuss.pytorch.org/) -- inference_mode, CUDA cache management
- [mp3lame-encoder](https://crates.io/crates/mp3lame-encoder), [pdf-extract](https://docs.rs/crate/pdf-extract/latest) -- crate evaluation

### Tertiary (LOW confidence)
- [Qwen3-TTS voice cloning blog post](https://ocdevel.com/blog/20260302-qwen-tts-voice-cloning) -- voice reuse pattern (needs API validation)
- [Qwen3-TTS hardware guide](https://deepwiki.com/mu-zi-lee/qwen3-tts-skill/8.2-memory-and-hardware-requirements) -- community documentation

---
*Research completed: 2026-03-27*
*Ready for roadmap: yes*
