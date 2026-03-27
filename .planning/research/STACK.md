# Technology Stack

**Project:** Chatter (Rust TTS CLI with PyO3 + Qwen3-TTS)
**Researched:** 2026-03-27

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
| Python | 3.12 | Runtime for qwen-tts | qwen-tts supports 3.9-3.13 but recommends 3.12 for optimal compatibility. PyO3 0.28 supports CPython 3.7+. | HIGH |
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

This is the most critical architectural decision. Key practices:

### Cargo.toml Setup

```toml
[dependencies]
pyo3 = { version = "0.28", features = ["auto-initialize"] }
```

### Key Patterns

1. **`auto-initialize` feature**: Starts Python interpreter on first `Python::attach()` call. Essential for CLI embedding.
2. **GIL management**: Use `Python::attach(|py| { ... })` to acquire the GIL. Release before any Rust mutex locks to avoid deadlocks.
3. **Error bridging**: PyO3 errors (`PyErr`) need conversion to `anyhow::Error` at the boundary. Use `thiserror` for typed Python bridge errors.
4. **One-time setup**: Initialize Python, import `qwen_tts`, and load the model once at startup. Cache the model object across CLI operations.
5. **Data transfer**: WAV audio comes back from Python as numpy arrays or bytes. Use PyO3's buffer protocol or extract as `Vec<f32>` / `Vec<u8>` for Rust-side MP3 encoding.

### Build Requirements

- System must have Python 3.12 development headers (`python3-dev` on Ubuntu)
- `pyo3-build-config` (transitive dep) auto-detects Python at build time
- Set `PYO3_PYTHON=python3.12` env var if multiple Python versions installed

## Audio Pipeline

```
qwen-tts (Python) -> WAV samples (f32/i16) -> hound (verify/normalize) -> mp3lame-encoder -> .mp3 file
```

The model outputs audio as numpy arrays (typically float32 at 24kHz). The pipeline:
1. Extract samples from Python as `Vec<f32>` via PyO3
2. Optionally write intermediate WAV with `hound` (useful for debugging)
3. Encode to MP3 with `mp3lame-encoder` at configurable bitrate (192kbps default)

## Installation

```toml
# Cargo.toml
[dependencies]
pyo3 = { version = "0.28", features = ["auto-initialize"] }
clap = { version = "4.5", features = ["derive"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
directories = "6"
indicatif = "0.18"
console = "0.15"
owo-colors = "4"
anyhow = "1"
thiserror = "2"
hound = "3.5"
mp3lame-encoder = "0.2"
pdf-extract = "0.10"
pulldown-cmark = "0.13"
```

```bash
# Python environment (user prerequisite)
pip install -U qwen-tts
# Optional but recommended:
pip install -U flash-attn --no-build-isolation
```

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
