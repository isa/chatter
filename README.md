# chatter

A Rust CLI tool that wraps [Qwen3-TTS](https://github.com/QwenLM/Qwen3-TTS) to provide text-to-speech with voice profile management. Design custom voices from natural language descriptions, clone voices from audio samples, and generate speech from text or documents — all from the terminal.

## Features

- **Voice Design** — Create voice profiles from natural language descriptions (e.g., "a warm, deep male voice with a slight British accent")
- **Voice Cloning** — Clone a voice from a reference audio sample
- **Speech Generation** — Generate MP3 audio from text, Markdown, PDF, or plain text files using saved voice profiles
- **Model Management** — Download, list, and manage Qwen3-TTS model variants (0.6B / 1.7B)
- **Environment Doctor** — Validate your setup (Python, GPU, dependencies) with `chatter doctor`

## Installation

```sh
brew install chatter
```

Chatter manages its own Python environment automatically. On first run, it creates a virtual environment and installs all Python dependencies — no manual setup needed.

### Requirements

- **Python** 3.12+ (installed automatically by Homebrew as a dependency)
- **GPU** — Apple Silicon (MLX/MPS) or CUDA-capable GPU

### Building from source

```sh
# Ensure Python 3.12 is available for PyO3
export PYO3_PYTHON=python3.12
cargo build --release
```

## Usage

```sh
# Check your environment
chatter doctor

# Download a model
chatter model download 1.7b

# Design a voice from a description
chatter design --name warm-narrator "A warm, calm male narrator voice"

# Clone a voice from audio
chatter clone --name my-voice reference.mp3

# Generate speech
chatter generate "Hello, world!" --profile warm-narrator -o output.mp3

# Generate from a file
chatter generate --file document.md --profile warm-narrator
```

## Supported Languages

Auto, Chinese, English, Japanese, Korean, French, German, Spanish, Portuguese, Russian, Italian

## Model Variants

| Model | Size | Use Case |
|-------|------|----------|
| Qwen3-TTS 1.7B VoiceDesign | ~3.4 GB | Voice design from descriptions |
| Qwen3-TTS 1.7B CustomVoice | ~3.4 GB | Named voice TTS |
| Qwen3-TTS 1.7B Base | ~3.4 GB | Voice cloning |
| Qwen3-TTS 0.6B CustomVoice | ~1.2 GB | Lighter named voice TTS |
| Qwen3-TTS 0.6B Base | ~1.2 GB | Lighter voice cloning |

## License

See [LICENSE](LICENSE) for details.
