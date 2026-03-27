# Phase 1: Foundation and Python Bridge - Research

**Researched:** 2026-03-27
**Domain:** Rust CLI scaffolding, PyO3 Python embedding, Qwen3-TTS model loading, environment validation
**Confidence:** MEDIUM-HIGH
**Sources:** 4 parallel research agents (pyo3-embedding, qwen-tts-api, cli-patterns, compute-backends)

## Summary

Phase 1 delivers a working Rust CLI binary (`chatter`) that initializes Python via PyO3, loads Qwen3-TTS models, validates the environment, and exposes all planned subcommands. Research covers four domains synthesized from parallel investigation.

**Key findings:**
1. PyO3 0.28 renamed `Python::with_gil()` to `Python::attach()`. Use `auto-initialize` feature only — NOT `extension-module`.
2. qwen-tts has 3 distinct generation methods requiring different model variants (VoiceDesign, CustomVoice, Base). VoiceDesign only exists as 1.7B.
3. MPS dtype gotcha: Base (clone) models MUST use float32 on MPS — float16 causes NaN errors.
4. Backend priority: CUDA > MLX > MPS > CPU (refuse). MLX preferred over MPS on Mac for better performance/memory.

## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| FOUN-01 | PyO3 bridge initializes Python runtime and loads qwen-tts models | PyO3 0.28.2 `auto-initialize`, `Python::attach()`, `Qwen3TTSModel.from_pretrained()` API |
| FOUN-02 | CLI binary parses subcommands with appropriate flags | clap 4.x derive macro, nested subcommands via enum |
| FOUN-03 | Language flag accepts 11 languages | Verified against qwen-tts: Chinese, English, Japanese, Korean, French, German, Spanish, Portuguese, Russian, Italian + Auto |
| FOUN-04 | Model size flag accepts 0.6B and 1.7B (default 1.7B) | Verified: 5 model variants across both sizes |
| FOUN-05 | Helpful error messages when GPU/Python unavailable | PyO3 `PyErr` handling, `is_instance_of::<PyImportError>()` pattern |
| UX-01 | Progress bar during model loading | indicatif `enable_steady_tick` + background thread works with blocking PyO3 calls |
| UX-04 | `--help` provides clear usage | clap derive auto-generates from doc comments |

## 1. PyO3 Embedding (from research/pyo3-embedding.md)

### Initialization
- Use `pyo3 = { version = "0.28", features = ["auto-initialize"] }` in Cargo.toml
- `Python::attach(|py| { ... })` is the current API (renamed from `with_gil` in 0.28)
- For chatter, may need manual `prepare_freethreaded_python()` if PYTHONPATH must be set before init
- **Never use `extension-module` feature** — that's for Python extensions, not embedding

### Calling qwen-tts from Rust
```rust
Python::attach(|py| {
    let model_class = py.import("qwen_tts")?.getattr("Qwen3TTSModel")?;
    let kwargs = PyDict::new(py);
    kwargs.set_item("device_map", "mps")?;
    kwargs.set_item("dtype", torch.getattr("float16")?)?;
    let model = model_class.call_method("from_pretrained",
        ("Qwen/Qwen3-TTS-12Hz-1.7B-CustomVoice",), Some(&kwargs))?;
    let model_handle: Py<PyAny> = model.unbind(); // cache across GIL boundaries
    Ok(())
})?;
```

### Caching Model Across Calls
- `Py<PyAny>` with `.unbind()` / `.bind(py)` for model persistence
- `GILOnceCell` for one-time module caching (e.g., imported torch module)

### Error Handling
- `thiserror` at engine boundary (`BridgeError` with `From<PyErr>`)
- `anyhow` at CLI layer for `.context()` messages
- Check exception types: `err.is_instance_of::<pyo3::exceptions::PyImportError>(py)`
- Extract tracebacks: `err.traceback(py).and_then(|tb| tb.format().ok())`

### Build Config
```toml
[dependencies]
pyo3 = { version = "0.28", features = ["auto-initialize"] }
```
Set `PYO3_PYTHON=python3.12` — system has Python 3.14 but qwen-tts needs 3.9-3.13.

### Audio Data Extraction
- Use `.tobytes()` + reinterpret for performance (avoids 720K Python float objects for 30s audio)
- Sample rate typically 24000 Hz

## 2. qwen-tts Python API (from research/qwen-tts-api.md)

### Model Variants

| Model ID | Size | Use Case |
|----------|------|----------|
| `Qwen/Qwen3-TTS-12Hz-1.7B-VoiceDesign` | 1.7B | Voice design from descriptions |
| `Qwen/Qwen3-TTS-12Hz-1.7B-CustomVoice` | 1.7B | Named voice TTS |
| `Qwen/Qwen3-TTS-12Hz-0.6B-CustomVoice` | 0.6B | Named voice TTS (smaller) |
| `Qwen/Qwen3-TTS-12Hz-1.7B-Base` | 1.7B | Voice cloning |
| `Qwen/Qwen3-TTS-12Hz-0.6B-Base` | 0.6B | Voice cloning (smaller) |

VoiceDesign only exists as 1.7B. 0.6B has significant quality degradation on long text.

### Three Generation Methods
1. `generate_voice_design(text, language, instruct)` — VoiceDesign model
2. `generate_voice_clone(text, language, voice_clone_prompt)` — Base model
3. `generate_custom_voice(text, speaker, language, instruct)` — CustomVoice model

All return iterables with `.audio` (numpy) and `.sr` (sample rate ~24000 Hz).

### Device/Dtype Matrix

| Backend | device_map | dtype | Notes |
|---------|-----------|-------|-------|
| CUDA | `"cuda:0"` | `torch.bfloat16` | Optional `attn_implementation="flash_attention_2"` |
| MPS | `"mps"` | `torch.float16` | **Base models MUST use float32** (NaN with float16) |
| CPU | `"cpu"` | `torch.float32` | Refuse — too slow for production |

### mlx-audio Alternative
- Supports Qwen3-TTS via `mlx_audio.tts.utils.load_model()`
- Different API, different model format (from `mlx-community/` HF org)
- Returns `mx.array` not numpy
- ~2-3x less memory than qwen-tts+MPS (unverified)
- **Uncertain:** voice cloning/design support — only `generate_custom_voice` confirmed

### Model Caching
- HF default: `~/.cache/huggingface/hub/models--Qwen--Qwen3-TTS-12Hz-{variant}/`
- Download without loading: `huggingface_hub.snapshot_download(repo_id="Qwen/...")`
- Check cached: `huggingface_hub.scan_cache_dir()`

## 3. CLI Patterns (from research/cli-patterns.md)

### Clap 4.x Structure
- Top-level `Cli` struct with `#[command(flatten)]` for `GlobalArgs`
- `Commands` enum with nested subcommands for `model` and `profiles`
- `ValueEnum` for `Language` (11 variants) and `ModelSize` (`#[value(name = "0.6b")]`)
- `global = true` must be on each `#[arg(...)]`, NOT on `#[command(flatten)]`

### indicatif Spinner with Elapsed Time
```rust
let pb = ProgressBar::new_spinner();
pb.set_style(
    ProgressStyle::with_template("{spinner:.cyan} {msg} ({elapsed})")
        .expect("valid template")
        .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"]),
);
pb.set_message("Loading Qwen3-TTS 1.7B...");
pb.enable_steady_tick(Duration::from_millis(100));
// ... blocking PyO3 call — spinner animates via background thread
pb.finish_with_message("Model loaded");
```

### owo-colors with NO_COLOR
```rust
use owo_colors::{OwoColorize, Stream};
"Error".if_supports_color(Stream::Stderr, |t| t.red().bold())
```
Requires `features = ["supports-colors"]`.

### Stub Subcommands
Define full struct (appears in `--help`), handler prints "coming in Phase 2" and exits cleanly.

## 4. Compute Backend Detection (from research/compute-backends.md)

### Detection Code
```python
# MPS
torch.backends.mps.is_available()

# CUDA
torch.cuda.is_available()
torch.cuda.get_device_name(0)
torch.cuda.get_device_properties(0).total_memory

# MLX
import mlx.core as mx
mx.metal.is_available()
mx.metal.device_info()  # {"memory_size": ..., "architecture": ...}
```

### Backend Priority
CUDA > MLX > MPS > CPU (refuse with error)

### Doctor Command
Single Python function `get_system_info()` returning flat dict:
- Use `importlib.metadata.version()` for package versions (avoids heavy imports)
- `shutil.disk_usage()` for disk space
- MPS has no VRAM — use `sysctl -n hw.memsize` for total RAM

### Memory Requirements (MEDIUM confidence)

| Model | CUDA VRAM | MPS RAM | MLX GPU budget |
|-------|-----------|---------|----------------|
| 0.6B  | 2-4 GB    | 8 GB    | 2-4 GB         |
| 1.7B  | 4-8 GB    | 16 GB   | 4-8 GB         |

## 5. Open Questions (Validate During Implementation)

1. `Python::attach()` vs `with_gil()` — verify which compiles in PyO3 0.28.2
2. mlx-audio voice cloning/design support — if missing, Mac must use qwen-tts+MPS for those
3. Python 3.12 compatibility — test `pip install qwen-tts` in clean 3.12 venv
4. qwen-tts progress callbacks — may be spinner-only (no progress percentage)
5. Sample rate — verify 24000 Hz across all variants

## 6. Recommended Stack (Phase 1)

| Library | Version | Purpose |
|---------|---------|---------|
| clap | 4.5+ | CLI parsing (derive + env features) |
| pyo3 | 0.28.2 | Python embedding (auto-initialize) |
| indicatif | 0.18.4 | Progress spinners |
| console | 0.15+ | Terminal utilities |
| owo-colors | 4.x | Colored output (supports-colors feature) |
| anyhow | 1.x | CLI-layer error handling |
| thiserror | 2.x | Engine-layer typed errors |
| serde | 1.x | Serialization |
| serde_json | 1.x | JSON format |
| directories | 6.0.0 | XDG paths |

---

*Phase: 01-foundation-and-python-bridge*
*Research completed: 2026-03-27 via 4 parallel research agents*
