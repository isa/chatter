# chatter

A Rust CLI tool that wraps [Qwen3-TTS](https://github.com/QwenLM/Qwen3-TTS) to provide text-to-speech with voice profile management. Design custom voices from natural language descriptions, clone voices from audio samples, and generate speech from text or documents — all from the terminal.

## Why chatter?

You have a 40-page PDF you need to review but no time to sit and read it. Drop it into chatter, pick a voice you like, and listen on your commute, during a workout, or while cooking dinner.

**Turn any document into a podcast.** PDFs, Word docs, Markdown notes — chatter chunks them intelligently and generates natural-sounding speech with proper pacing between sections.

**Your voice, your way.** Design a voice from a description ("a calm British narrator in his 50s") or clone one from a short audio clip. Save it as a profile and reuse it across everything you generate.

**Fits into your workflow.** chatter is a CLI tool, which means it composes with everything:

```sh
# Convert a doc and listen while you work
chatter generate --file report.pdf --profile narrator --no-play -o report.mp3

# Batch-convert a folder of markdown notes
for f in notes/*.md; do chatter generate --file "$f" --profile narrator -o "${f%.md}.mp3"; done

# Pipe text from another command
pbpaste | chatter generate --profile narrator -o clipboard.mp3

# macOS Shortcut: speak selected text from any app
# Create a Shortcut that runs: chatter generate "$selected_text" --profile narrator
```

**Runs locally.** No cloud API, no subscription, no data leaving your machine. Your documents stay private.

## Features

- **Voice Design** — Create voice profiles from natural language descriptions (e.g., "a warm, deep male voice with a slight British accent")
- **Voice Cloning** — Clone a voice from a reference audio sample
- **Speech Generation** — Generate MP3 audio from text using saved voice profiles
- **Model Management** — Download, list, and manage Qwen3-TTS model variants
- **Document Input** — Generate speech from PDF, DOCX, TXT, and Markdown files with automatic text chunking
- **Environment Doctor** — Validate your setup with `chatter doctor` and auto-fix with `--fix`

## Installation

```sh
brew install chatter
```

Homebrew installs everything: the binary, a bundled Python venv with the correct inference backend (`mlx-audio` on Apple Silicon, `qwen-tts` on CUDA), and all Python dependencies. No manual setup needed.

After installing, download the TTS models:

```sh
chatter model download                   # 8-bit (default, ~6 GB total)
chatter model download --variant bf16    # full precision (~12 GB total)
```

### Requirements

- **Python** 3.12+ (installed automatically by Homebrew as a dependency)
- **GPU** — Apple Silicon (MLX) or CUDA-capable GPU
- **Disk** — ~6 GB for 8-bit models (default) or ~12 GB for bf16

## Usage

```sh
# Check your environment
chatter doctor

# Download models (required before first use)
chatter model download

# Design a voice from a description
chatter design "A warm, calm male narrator voice"

# Clone a voice from audio
chatter clone reference.mp3

# Generate speech
chatter generate "Hello, world!" --profile warm-narrator -o output.mp3

# Generate from a document
chatter generate --file document.pdf --profile warm-narrator -o output.mp3

# Split long documents into separate files per chunk
chatter generate --file book.md --profile warm-narrator --split

# Adjust speech speed (0.5x to 3.0x)
chatter generate "Hello, world!" --profile warm-narrator --speed 1.2

# Generate without auto-playing audio
chatter generate "Hello!" --profile warm-narrator --no-play

# List saved profiles
chatter profiles list
```

### Voice Design Flow

```sh
chatter design "warm and calming motherly sound of a british female in her 60s"
```

1. Generates a voice preview and plays it
2. Interactive menu: accept, retry, change description, or quit
3. On accept, prompts for a profile name (with auto-suggested default)
4. Saves the profile to `~/.config/chatter/profiles/<name>/`

### Doctor

```sh
# Diagnose issues (read-only)
chatter doctor

# Auto-fix: downloads missing models
chatter doctor --fix
```

## File Layout

```
~/.config/chatter/
  profiles/
    warm-narrator/
      profile.toml        # metadata (name, type, language, description)
      sample.mp3           # cached preview audio
      ref_audio.wav        # reference audio (MLX) or voice_prompt.bin (CUDA)

# Venv is bundled by Homebrew in the Cellar (not in user home)
$(brew --prefix)/Cellar/chatter/<version>/libexec/venv/
```

## Supported Languages

Auto, Chinese, English, Japanese, Korean, French, German, Spanish, Portuguese, Russian, Italian

## Model Variants

Downloads default to 8-bit quantized models (smaller, faster). Use `--variant bf16` for full precision.

| Model | 8-bit | bf16 | Use Case |
|-------|-------|------|----------|
| Qwen3-TTS 1.7B VoiceDesign | ~1.7 GB | ~3.4 GB | Voice design from descriptions |
| Qwen3-TTS 1.7B CustomVoice | ~1.7 GB | ~3.4 GB | Speech generation with saved profiles |
| Qwen3-TTS 1.7B Base | ~1.7 GB | ~3.4 GB | Voice cloning |

On Apple Silicon, MLX-optimized variants are used automatically. Override at inference time with `--variant bf16` if needed.

## Development

### Prerequisites

- Rust 1.85+ (edition 2024)
- [mise](https://mise.jdx.dev/) (recommended) or Python 3.12+ installed manually
- Apple Silicon Mac or CUDA GPU

### Building from source

```sh
# Install pinned tool versions (Python 3.12.3)
mise install

# Build
cargo build --release
```

`mise` reads `mise.toml` in the project root, which pins Python 3.12.3 and sets `PYO3_PYTHON` so PyO3 links against the correct interpreter automatically.

If you don't use mise, set `PYO3_PYTHON` manually:

```sh
PYO3_PYTHON=python3.12 cargo build --release
```

### Setting up the dev venv

The binary discovers its Python venv via `CHATTER_VENV` env var or by looking for `../libexec/venv/` relative to itself (Homebrew layout). For development, create and point to your own venv:

```sh
# Create venv
python3 -m venv ~/.config/chatter/dev-venv
~/.config/chatter/dev-venv/bin/pip install mlx-audio  # Apple Silicon
# or: ~/.config/chatter/dev-venv/bin/pip install qwen-tts  # CUDA

# Tell chatter where to find it
export CHATTER_VENV=~/.config/chatter/dev-venv

# Verify
target/release/chatter doctor
```

The `chatter_bridge.py` adapter module is embedded in the binary at compile time (`include_str!`) and auto-installed into the venv's site-packages on first run. After changing `chatter_bridge.py`, rebuild and run any command to update it.

### Architecture

```
src/
  main.rs              # CLI entry point, venv discovery
  cli.rs               # clap argument definitions
  ui.rs                # spinners, doctor output helpers
  chunk.rs             # text chunking with pause markers
  bridge/
    mod.rs             # re-exports
    venv.rs            # venv discovery, Python configuration
    runtime.rs         # GPU/backend detection (CUDA > MLX > MPS > CPU)
    inference.rs       # PyO3 calls to chatter_bridge.py
    model.rs           # HuggingFace model download/list/remove
    doctor.rs          # system diagnostics gathering
    error.rs           # BridgeError types
  commands/
    design.rs          # `chatter design` — interactive voice creation
    clone.rs           # `chatter clone` — voice cloning from audio
    generate.rs        # `chatter generate` — speech synthesis
    profiles.rs        # `chatter profiles list|show|delete`
    model.rs           # `chatter model download|list|remove`
    doctor.rs          # `chatter doctor` — environment check
  extract/
    mod.rs             # trait + format dispatch
    pdf.rs             # PDF text extraction (pdf-extract)
    docx.rs            # DOCX text extraction
    markdown.rs        # Markdown to plain text (pulldown-cmark)
    txt.rs             # Plain text passthrough
  profile/
    mod.rs             # ProfileMetadata, ProfileInfo types
    storage.rs         # TOML-based profile CRUD
  audio/
    mod.rs             # WAV-to-MP3 encoding (mp3lame-encoder)
    playback.rs        # afplay/paplay shell-out
    time_stretch.rs    # WSOLA time-stretching for --speed flag
chatter_bridge.py      # Python adapter (embedded at compile time)
```

## License

See [LICENSE](LICENSE) for details.
