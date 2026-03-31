<!-- GSD:project-start source:PROJECT.md -->
## Project

**Chatter**

A Rust CLI tool that wraps Qwen3-TTS from Hugging Face to provide text-to-speech capabilities with voice profile management. Users can design custom voices from natural language descriptions, clone voices from audio samples, and generate speech from text or documents — all from the terminal with progress feedback.

**Core Value:** Users can create reusable voice profiles and generate high-quality speech from text or documents without leaving the command line.

### Constraints

- **Tech stack**: Rust CLI with PyO3 for Python interop — required because model is Python-only
- **Hardware**: Apple Silicon (MLX/MPS) or CUDA-capable GPU required for local inference
- **Distribution**: `brew install chatter` must work out of the box. Chatter manages its own Python venv at `~/.local/share/chatter/venv/` with auto-setup on first run. Users do NOT manually install Python packages.
- **Dependencies**: Python 3.13 runtime (Homebrew formula declares `depends_on "python@3.13"`). `qwen-tts` is auto-installed into the managed venv on first run. Python 3.13 aligns ChatterBox (`chatterbox-tts`) with NumPy 2.x pins (PyPI allows NumPy 2.x only for Python ≥3.13).
- **Audio**: MP3 output (requires encoding from WAV produced by model)
<!-- GSD:project-end -->

<!-- GSD:stack-start source:research/STACK.md -->
## Technology Stack

## Recommended Stack
### Core Framework
| Technology | Version | Purpose | Why | Confidence |
|------------|---------|---------|-----|------------|
| Rust (edition 2024) | 1.85+ | Language | Systems-level performance, strong type system, excellent CLI ecosystem. PyO3 0.28 requires Rust >= 1.83. | HIGH |
| PyO3 | 0.28.2 | Python embedding | The only mature Rust-Python interop crate. Actively maintained (released 2026-02-18). Enables calling `qwen_tts` Python package directly from Rust without subprocess overhead. Use `auto-initialize` feature. | HIGH |
| clap | 4.5+ | CLI argument parsing | De facto standard for Rust CLIs. Derive macro API eliminates boilerplate. Powers ripgrep, bat, fd. | HIGH |
### Audio Processing
| Technology | Version | Purpose | Why | Confidence |
|------------|---------|---------|-----|------------|
| hound | 3.5.1 | WAV reading | Standard Rust WAV library. 7.5M+ downloads. Reads WAV output from qwen-tts model inference before MP3 encoding. | HIGH |
| mp3lame-encoder | 0.2.2 | WAV-to-MP3 encoding | High-level safe Rust bindings to LAME. Statically links LAME so no runtime dependency. The most ergonomic MP3 encoding option in Rust. | MEDIUM |
### File Parsing
| Technology | Version | Purpose | Why | Confidence |
|------------|---------|---------|-----|------------|
| pdf-extract | 0.10.0 | PDF text extraction | Purpose-built for text extraction (not PDF manipulation). Simpler API than lopdf for our read-only use case. | MEDIUM |
| pulldown-cmark | 0.13.3 | Markdown parsing | CommonMark-compliant, streaming parser. Very fast, no AST allocation needed -- we just need to strip markup and extract plain text. Released 2026-03-22. | HIGH |
### Terminal UX
| Technology | Version | Purpose | Why | Confidence |
|------------|---------|---------|-----|------------|
| indicatif | 0.18.4 | Progress bars | The standard Rust progress bar library. Supports bounded bars (file processing) and spinners (model loading). MultiProgress for concurrent operations. Released 2026-02-14. | HIGH |
| console | 0.15+ | Terminal utilities | Sister crate to indicatif (same `console-rs` org). Handles terminal width detection, styling, and ANSI support. | HIGH |
| owo-colors | 4.x | Colored output | Zero-allocation, no_std-compatible terminal coloring. Respects NO_COLOR/FORCE_COLOR env vars. Recommended by Rust CLI best practices guide over `colored` crate. | HIGH |
### Configuration & Serialization
| Technology | Version | Purpose | Why | Confidence |
|------------|---------|---------|-----|------------|
| serde | 1.x | Serialization framework | De facto standard. Required by every config/data format crate. Use `derive` feature. | HIGH |
| serde_json | 1.x | JSON serialization | Voice profile metadata storage format. Human-readable, easy to debug. | HIGH |
| toml | 0.8+ | TOML config files | Optional: if app-level config is needed beyond voice profiles. Idiomatic in Rust ecosystem. | HIGH |
| directories | 6.0.0 | XDG directory paths | Cross-platform (Linux/macOS/Windows) standard directory resolution. Returns `~/.config/chatter/` on Linux, `~/Library/Application Support/` on macOS. Actively maintained under `xdg-rs` org. | HIGH |
### Error Handling
| Technology | Version | Purpose | Why | Confidence |
|------------|---------|---------|-----|------------|
| anyhow | 1.x | Application error handling | For main.rs and top-level CLI code. Ergonomic error context with `.context()`. Standard for Rust CLI apps. | HIGH |
| thiserror | 2.x | Typed error definitions | For internal library modules (PyO3 bridge, audio encoding). Gives callers structured error types to match on. | HIGH |
### Python Environment
| Technology | Version | Purpose | Why | Confidence |
|------------|---------|---------|-----|------------|
| Python | 3.13 | Runtime for qwen-tts + ChatterBox | Shared venv uses NumPy 2.3 with SciPy 1.16; `chatterbox-tts` requires Python 3.13+ for NumPy 2.x. PyO3 0.28 supports CPython 3.7+. | HIGH |
| qwen-tts | 0.1.1 | TTS model inference | Official Python package from Qwen team. Wraps Qwen3-TTS models. Depends on PyTorch, transformers, soundfile. | HIGH |
## Alternatives Considered
| Category | Recommended | Alternative | Why Not |
|----------|-------------|-------------|---------|
| Python interop | PyO3 (embedding) | `std::process::Command` (subprocess) | Subprocess has higher latency per call, no shared state between calls, harder error handling, no progress callbacks. PyO3 keeps Python alive in-process. |
| Python interop | PyO3 (embedding) | Rewrite model inference in Rust | Qwen3-TTS depends on PyTorch + transformers. Rewriting in Rust would be a massive effort with no clear benefit. |
| MP3 encoding | mp3lame-encoder | `symphonia` | Symphonia is primarily a decoder. No MP3 encoding support. |
| MP3 encoding | mp3lame-encoder | FFmpeg subprocess | External dependency, harder to distribute, more failure modes. LAME static linking is cleaner. |
| PDF parsing | pdf-extract | lopdf | lopdf is a PDF manipulation library (read/write/edit). We only need text extraction. pdf-extract is purpose-built for this. |
| PDF parsing | pdf-extract | `pdf-rs/pdf` | Less mature, fewer downloads. pdf-extract is the established choice for text extraction. |
| Markdown parsing | pulldown-cmark | comrak | comrak builds a full AST and supports GFM extensions. We only need to strip markup to get plain text -- pulldown-cmark's streaming approach is lighter and faster. |
| Progress bars | indicatif | `pbr` | pbr is unmaintained. indicatif is the ecosystem standard. |
| Colored output | owo-colors | `colored` | `colored` allocates strings. owo-colors is zero-allocation and respects NO_COLOR standard. |
| Directory paths | directories | `dirs` | `dirs` is the low-level sibling. `directories` provides `ProjectDirs` which gives us app-scoped paths (config, data, cache) in one call. |
| Config format | JSON (serde_json) | TOML / YAML | Voice profiles are data, not human-edited config. JSON is simpler, universally understood, and has the best tooling. TOML for app config if needed later. |
## PyO3 Integration Strategy
### Cargo.toml Setup
### Key Patterns
### Build Requirements
- System must have Python 3.13 development headers (`python3-dev` on Ubuntu)
- `pyo3-build-config` (transitive dep) auto-detects Python at build time
- Set `PYO3_PYTHON=python3.13` env var if multiple Python versions installed
## Audio Pipeline
## Installation
# Cargo.toml
# Python environment (user prerequisite)
# Optional but recommended:
## What NOT to Use
| Technology | Why Not |
|------------|---------|
| `tokio` / `async-std` | This is a synchronous CLI tool. Model inference is blocking (GPU-bound). Async adds complexity with zero benefit here. |
| `reqwest` / `ureq` | No network requests needed. Local-only inference, local profiles. |
| `rusqlite` / `sqlx` | Profiles are individual JSON files, not a database. SQLite is overkill for < 100 profiles. |
| `crossterm` / `ratatui` | No TUI needed. This is a straightforward CLI, not an interactive terminal app. |
| `rodio` / `cpal` | No audio playback. Generate-to-file only (out of scope per PROJECT.md). |
| `image` / `resvg` | No image processing needed. |
| Python virtualenv management | Out of scope. User is responsible for having `qwen-tts` installed. Document the prerequisite, don't automate it. |
## Sources
- [PyO3 0.28.2 on docs.rs](https://docs.rs/crate/pyo3/latest) - Verified version 0.28.2 (2026-02-18)
- [PyO3 User Guide](https://pyo3.rs/) - Embedding patterns, auto-initialize feature
- [PyO3 GitHub](https://github.com/PyO3/pyo3) - Rust 1.83+ requirement
- [clap on crates.io](https://crates.io/crates/clap) - CLI framework
- [indicatif 0.18.4 on docs.rs](https://docs.rs/crate/indicatif/latest) - Verified version (2026-02-14)
- [mp3lame-encoder on crates.io](https://crates.io/crates/mp3lame-encoder) - MP3 encoding
- [pdf-extract 0.10.0 on docs.rs](https://docs.rs/crate/pdf-extract/latest) - Verified version (2025-10-03)
- [pulldown-cmark 0.13.3 on docs.rs](https://docs.rs/crate/pulldown-cmark/latest) - Verified version (2026-03-22)
- [directories 6.0.0 on docs.rs](https://docs.rs/crate/directories/latest) - Verified version (2025-01-12)
- [hound 3.5.1 on docs.rs](https://docs.rs/crate/hound/latest) - WAV library
- [qwen-tts 0.1.1 on PyPI](https://pypi.org/project/qwen-tts/) - Verified version (2026-02-06)
- [Qwen3-TTS GitHub](https://github.com/QwenLM/Qwen3-TTS) - Model documentation
- [owo-colors on lib.rs](https://lib.rs/crates/owo-colors) - Terminal coloring
- [Rain's Rust CLI Recommendations](https://rust-cli-recommendations.sunshowers.io/managing-colors-in-rust.html) - Color management best practices
- [anyhow / thiserror best practices](https://dev.to/leapcell/rust-error-handling-compared-anyhow-vs-thiserror-vs-snafu-2003) - Error handling patterns
<!-- GSD:stack-end -->

<!-- GSD:conventions-start source:CONVENTIONS.md -->
## Conventions

Conventions not yet established. Will populate as patterns emerge during development.
<!-- GSD:conventions-end -->

<!-- GSD:architecture-start source:ARCHITECTURE.md -->
## Architecture

Architecture not yet mapped. Follow existing patterns found in the codebase.
<!-- GSD:architecture-end -->

<!-- GSD:workflow-start source:GSD defaults -->
## GSD Workflow Enforcement

Before using Edit, Write, or other file-changing tools, start work through a GSD command so planning artifacts and execution context stay in sync.

Use these entry points:
- `/gsd:quick` for small fixes, doc updates, and ad-hoc tasks
- `/gsd:debug` for investigation and bug fixing
- `/gsd:execute-phase` for planned phase work

Do not make direct repo edits outside a GSD workflow unless the user explicitly asks to bypass it.
<!-- GSD:workflow-end -->



<!-- GSD:profile-start -->
## Developer Profile

> Profile not yet configured. Run `/gsd:profile-user` to generate your developer profile.
> This section is managed by `generate-claude-profile` -- do not edit manually.
<!-- GSD:profile-end -->
