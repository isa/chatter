# chatter

A Rust CLI tool for local text-to-speech with voice profile management. It wraps **[Qwen3-TTS](https://github.com/QwenLM/Qwen3-TTS)** and, on supported hardware, **[ChatterBox](https://github.com/resemble-ai/chatterbox)** (Resemble AI) as a second engine. Design voices from descriptions, clone from audio, and generate speech from text or documents — all from the terminal.

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

- **Dual engines** — `qwen` (default) or `chatterbox` via `--engine` or `CHATTER_ENGINE`
- **Voice Design** — Create profiles from natural language (Qwen3-TTS)
- **Voice Cloning** — Clone from reference audio (both engines; profiles record which engine created them)
- **Speech Generation** — MP3 output from text or documents
- **Model Management** — Download and list Qwen3-TTS and ChatterBox model caches
- **Document Input** — PDF, DOCX, TXT, Markdown with chunking
- **Environment Doctor** — `chatter doctor` checks venv, GPU, models, and Python import sanity; `chatter doctor --fix` can install missing deps and download models

## Installation

```sh
brew tap isa/tap
brew install chatter
```

Homebrew installs the Rust binary and a **bundled Python 3.13 venv** with the right inference stack (`mlx-audio` + pinned deps on Apple Silicon, `qwen-tts` on CUDA). ChatterBox Python deps are installed when you run `chatter model download --engine chatterbox` or `chatter doctor --fix` (they are not all bundled in the initial Cellar venv on every platform). Python 3.13 is required so the same NumPy 2.x pins work for both MLX/Qwen and `chatterbox-tts` (that package only allows NumPy 2.x on Python 3.13+).

**PATH:** If `which chatter` shows `~/.cargo/bin/chatter`, you are running a **cargo-built** binary, not Homebrew. Use `$(brew --prefix)/bin/chatter` or put Homebrew’s `bin` before `~/.cargo/bin` in `PATH`.

### Install Speed + Output Noise

- Homebrew controls how `brew install` looks in your terminal; formulas cannot inject a custom progress bar.
- Avoid `brew install --verbose` unless debugging (it prints full Cargo/pip output).
- For **local formula testing** from this repo, use `scripts/brew-test-local.sh`: quiet by default, spinner while install runs, full log saved under `/tmp/chatter-brew-test/`. Flags: `--verbose`, `--audit`, `--runtime-bundle PATH`.

**Optional fast path (maintainers / CI):** ship a **prebuilt runtime venv** tarball to skip the long `pip install -r requirements-mlx.txt` step during formula build.

1. Build the bundle (Apple Silicon / `requirements-mlx.txt`):

```sh
./scripts/build-runtime-bundle.sh
# writes ${TMPDIR:-/tmp}/chatter-runtime-bundle/chatter-runtime-venv-macos-arm64.tar.gz
```

2. Test install using the bundle:

```sh
./scripts/brew-test-local.sh --runtime-bundle "${TMPDIR:-/tmp}/chatter-runtime-bundle/chatter-runtime-venv-macos-arm64.tar.gz"
```

3. In release builds, set `CHATTER_RUNTIME_BUNDLE_URL` to an `https://` URL for that tarball so `brew install` can download and extract it during `install`.

### Download models (after install)

```sh
chatter model download                    # Qwen3-TTS (8-bit default, ~6 GB total)
chatter model download --variant bf16     # Qwen bf16 (~12 GB total)

chatter model download --engine chatterbox   # ChatterBox deps + model variants (large; Apple Silicon uses MLX community builds where available)
```

Run `chatter doctor` to confirm imports, GPU, and caches.

### Requirements

- **Python** 3.13 (Homebrew `python@3.13` when using the formula; matches the bundled venv)
- **GPU** — Apple Silicon (MLX / MPS) or CUDA-capable GPU
- **Disk** — multiple GB for Qwen models; additional space for ChatterBox variants if you use `--engine chatterbox` (see `chatter model list`)

## Usage

```sh
# Check your environment
chatter doctor

# Download models (required before first use)
chatter model download

# Design a voice from a description
chatter design "A warm, calm male narrator voice"

# Clone a voice from audio (default engine: qwen)
chatter clone reference.mp3

# ChatterBox clone / generate (after: model download --engine chatterbox)
chatter clone --engine chatterbox reference.wav --name myvoice
chatter generate --engine chatterbox "Hello" --profile myvoice -o out.mp3

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
# Diagnose venv, Python imports (numpy/scipy, etc.), GPU, and cached models
chatter doctor

# Auto-fix: install ChatterBox deps, download missing models, repair common issues
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

**Qwen3-TTS** downloads default to 8-bit quantized models (smaller, faster). Use `--variant bf16` for full precision.

| Model | 8-bit | bf16 | Use Case |
|-------|-------|------|----------|
| Qwen3-TTS 1.7B VoiceDesign | ~1.7 GB | ~3.4 GB | Voice design from descriptions |
| Qwen3-TTS 1.7B CustomVoice | ~1.7 GB | ~3.4 GB | Speech generation with saved profiles |
| Qwen3-TTS 1.7B Base | ~1.7 GB | ~3.4 GB | Voice cloning |

On Apple Silicon, MLX-optimized Qwen variants are used when available; use `--variant bf16` to override.

**ChatterBox** uses separate Hugging Face repos (Original, Turbo, Multilingual, plus MLX community builds on Apple Silicon). Sizes vary — use `chatter model list` after downloading.

## Development

### Prerequisites

- Rust 1.85+ (edition 2024)
- [mise](https://mise.jdx.dev/) (recommended) or Python 3.13+ installed manually
- Apple Silicon Mac or CUDA GPU

### Building from source

```sh
# Install pinned tool versions (Python 3.13.x — see mise.toml)
mise install

# Build
cargo build --release
```

`mise` reads `mise.toml` in the project root, which pins Python and sets `PYO3_PYTHON` so PyO3 links against the correct interpreter automatically.

If you don't use mise, set `PYO3_PYTHON` manually:

```sh
PYO3_PYTHON=python3.13 cargo build --release
```

### Setting up the dev venv

The binary discovers its Python venv via `CHATTER_VENV` env var or by looking for `../libexec/venv/` relative to itself (Homebrew layout). For development, create and point to your own venv:

```sh
# Create venv
python3 -m venv ~/.config/chatter/dev-venv
~/.config/chatter/dev-venv/bin/pip install -r requirements-mlx.txt   # Apple Silicon (pinned)
# or: ~/.config/chatter/dev-venv/bin/pip install qwen-tts              # CUDA
# For ChatterBox: pip install chatterbox-tts (see also install_chatterbox_deps in bridge)

# Tell chatter where to find it
export CHATTER_VENV=~/.config/chatter/dev-venv

# Verify
target/release/chatter doctor
```

The `chatter_bridge/` Python package is copied into the venv’s site-packages (Homebrew pre-installs it; dev venvs get it on first run). Rebuild after changing bridge code.

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
    doctor.rs          # system diagnostics + import sanity checks
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
chatter_bridge/        # Python bridge package (engines/qwen, engines/chatterbox, …)
```

## Releasing (maintainers)

1. Tag the release from `main` (match `version` in `Cargo.toml`), e.g. `git tag v1.1.3 && git push origin v1.1.3`.
2. If Homebrew reports a checksum mismatch for `Formula/chatter.rb`, refresh `sha256` with the GitHub tarball (not `git archive`):

```sh
curl -sL "https://github.com/isa/chatter/archive/refs/tags/vX.Y.Z.tar.gz" | shasum -a 256
```

3. Commit the updated `sha256` if it changed.

## License

See [LICENSE](LICENSE) for details.
