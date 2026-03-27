# Architecture Research

**Domain:** Rust CLI with embedded Python (PyO3) for ML inference (TTS)
**Researched:** 2026-03-27
**Confidence:** MEDIUM-HIGH

## Standard Architecture

### System Overview

```
┌──────────────────────────────────────────────────────────────────┐
│                        CLI Layer (Rust)                          │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐        │
│  │ design   │  │ clone    │  │ generate  │  │ profiles │        │
│  │ command  │  │ command  │  │ command   │  │ command  │        │
│  └────┬─────┘  └────┬─────┘  └────┬─────┘  └────┬─────┘        │
│       │              │             │              │              │
├───────┴──────────────┴─────────────┴──────────────┴──────────────┤
│                    Orchestration Layer (Rust)                     │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────────────┐   │
│  │ ProfileStore │  │ ProgressMgr  │  │ AudioPostProcessor   │   │
│  └──────┬───────┘  └──────┬───────┘  └──────────┬───────────┘   │
│         │                 │                      │               │
├─────────┴─────────────────┴──────────────────────┴───────────────┤
│                    Python Bridge (PyO3 boundary)                  │
│  ┌──────────────────────────────────────────────────────────┐    │
│  │                   InferenceEngine                         │    │
│  │  - Python interpreter lifecycle                           │    │
│  │  - GIL acquisition/release                                │    │
│  │  - Model loading & caching                                │    │
│  │  - Progress callback relay                                │    │
│  └──────────────────────────────────────────────────────────┘    │
├──────────────────────────────────────────────────────────────────┤
│                    Python Runtime (embedded)                      │
│  ┌────────────┐  ┌────────────┐  ┌──────────────────────┐       │
│  │ qwen_tts   │  │ torch      │  │ soundfile            │       │
│  └────────────┘  └────────────┘  └──────────────────────┘       │
├──────────────────────────────────────────────────────────────────┤
│                    Storage Layer (filesystem)                     │
│  ┌──────────────────┐  ┌──────────────────┐                     │
│  │ Profile Store     │  │ Audio Files       │                    │
│  │ ~/.config/chatter │  │ (WAV/MP3 output)  │                    │
│  └──────────────────┘  └──────────────────┘                     │
└──────────────────────────────────────────────────────────────────┘
```

### Component Responsibilities

| Component | Responsibility | Typical Implementation |
|-----------|----------------|------------------------|
| CLI Commands | Parse args, validate input, dispatch to orchestration | `clap` derive macros with subcommand enum |
| ProfileStore | CRUD for voice profiles, metadata + cached audio | Rust structs serialized to TOML + audio files on disk |
| ProgressManager | Receive progress callbacks from Python, render bars | `indicatif` progress bars fed by PyO3 callback relay |
| AudioPostProcessor | Convert WAV output from model to MP3 | Pure Rust MP3 encoding (e.g., `mp3lame-encoder` or `symphonia`) |
| InferenceEngine | Bridge between Rust orchestration and Python model | PyO3 `Python::with_gil` calls into `qwen_tts` package |
| Python Runtime | Actual ML inference (model load, generate) | Embedded CPython with `qwen_tts`, `torch`, `soundfile` |

## Recommended Project Structure

```
src/
├── main.rs                # Entry point: init Python, parse CLI, dispatch
├── cli/                   # CLI layer
│   ├── mod.rs             # Top-level Cli struct with clap derive
│   ├── design.rs          # `chatter design` subcommand
│   ├── clone.rs           # `chatter clone` subcommand
│   ├── generate.rs        # `chatter generate` subcommand
│   └── profiles.rs        # `chatter profiles list/show/delete`
├── engine/                # Python bridge layer
│   ├── mod.rs             # InferenceEngine struct + init
│   ├── models.rs          # Model variant enum, loading logic
│   ├── voice_design.rs    # PyO3 calls for generate_voice_design()
│   ├── voice_clone.rs     # PyO3 calls for generate_voice_clone()
│   ├── custom_voice.rs    # PyO3 calls for generate_custom_voice()
│   └── progress.rs        # Progress callback relay (Python -> Rust)
├── profile/               # Profile management
│   ├── mod.rs             # ProfileStore
│   ├── types.rs           # Profile, ProfileMetadata structs
│   └── storage.rs         # Filesystem operations, XDG paths
├── audio/                 # Audio post-processing
│   ├── mod.rs
│   └── encode.rs          # WAV -> MP3 conversion
└── error.rs               # Unified error types across boundaries
```

### Structure Rationale

- **cli/:** Thin layer -- each file maps 1:1 to a subcommand. Contains no business logic, only argument parsing and dispatch. This keeps the CLI testable independently of Python.
- **engine/:** The critical boundary. All PyO3 interaction is isolated here. No other module touches `pyo3` directly. This makes the Python dependency explicit and contained.
- **profile/:** Pure Rust, no Python dependency. Can be developed and tested independently. Uses standard serde for serialization.
- **audio/:** Pure Rust audio encoding. Decoupled from inference so it can be tested with fixture WAV files.
- **error.rs:** Single error enum that maps Python exceptions, IO errors, and domain errors into a unified type.

## Architectural Patterns

### Pattern 1: Single Python Interpreter with Lazy Model Loading

**What:** Initialize the Python interpreter once at program start. Load models lazily on first use, then cache them in a `Py<PyAny>` handle for reuse across operations.

**When to use:** Always -- this is the only sane approach for ML model lifecycle. Model loading takes 10-30 seconds; you cannot afford to reload per invocation.

**Trade-offs:** Simpler lifecycle management. Models consume GPU memory for the duration of the process. For a CLI tool (short-lived process), this is fine.

**Example:**
```rust
use pyo3::prelude::*;
use pyo3::types::PyDict;

pub struct InferenceEngine {
    model: Option<Py<PyAny>>,  // Cached model reference (GIL-independent)
    model_id: String,
}

impl InferenceEngine {
    pub fn new(model_id: &str) -> Self {
        // Initialize Python interpreter once
        pyo3::prepare_freethreaded_python();
        Self {
            model: None,
            model_id: model_id.to_string(),
        }
    }

    fn ensure_model(&mut self) -> PyResult<()> {
        if self.model.is_some() {
            return Ok(());
        }
        Python::with_gil(|py| {
            let qwen_tts = py.import("qwen_tts")?;
            let model_class = qwen_tts.getattr("Qwen3TTSModel")?;

            let kwargs = PyDict::new(py);
            kwargs.set_item("device_map", "cuda:0")?;
            // dtype and attn_implementation set via torch import
            let torch = py.import("torch")?;
            kwargs.set_item("dtype", torch.getattr("bfloat16")?)?;
            kwargs.set_item("attn_implementation", "flash_attention_2")?;

            let model = model_class.call_method("from_pretrained",
                (&self.model_id,), Some(&kwargs))?;

            self.model = Some(model.unbind());  // Store as Py<PyAny>
            Ok(())
        })
    }
}
```

**Key detail:** Use `Py<PyAny>` (not `Bound<'py, PyAny>`) to store the model reference outside of `with_gil` scope. `Py<T>` is the GIL-independent smart pointer that lets you hold Python objects across GIL acquisitions.

### Pattern 2: Progress Callback via PyCFunction Closure

**What:** Create a Rust closure wrapped as a Python callable, pass it into the Python generation call, and have it update an `indicatif` progress bar.

**When to use:** For all inference operations (they take seconds to minutes).

**Trade-offs:** Requires GIL to be held when the callback fires (it will be, since Python calls it). The callback must be lightweight to avoid slowing inference.

**Example:**
```rust
use pyo3::prelude::*;
use pyo3::types::PyCFunction;
use indicatif::ProgressBar;
use std::sync::Arc;

fn create_progress_callback(py: Python<'_>, bar: Arc<ProgressBar>)
    -> PyResult<Bound<'_, PyCFunction>>
{
    PyCFunction::new_closure(
        py,
        None,  // name
        None,  // doc
        move |args: &Bound<'_, PyTuple>, _kwargs: Option<&Bound<'_, PyDict>>| -> PyResult<()> {
            let step: u64 = args.get_item(0)?.extract()?;
            let total: u64 = args.get_item(1)?.extract()?;
            bar.set_length(total);
            bar.set_position(step);
            Ok(())
        },
    )
}
```

**Important caveat:** The qwen-tts package may not expose a progress callback parameter natively. If it does not, the alternative is to monkey-patch or wrap the `transformers` `generate()` call with a custom `GenerationConfig` or `LogitsProcessor` that calls back on each token. This needs validation during implementation.

### Pattern 3: Error Boundary at the Engine Layer

**What:** All Python exceptions are caught and converted to Rust error types at the engine boundary. No `PyErr` leaks above the `engine/` module.

**When to use:** Always. This is the most important architectural boundary in the project.

**Trade-offs:** Requires mapping Python exception types to meaningful Rust errors. Some Python tracebacks may be lost -- preserve them in error messages.

**Example:**
```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ChatterError {
    #[error("Python runtime error: {message}\n{traceback}")]
    PythonError {
        message: String,
        traceback: String,
    },

    #[error("Model not found: {model_id}")]
    ModelNotFound { model_id: String },

    #[error("CUDA not available -- GPU required for inference")]
    CudaUnavailable,

    #[error("Profile not found: {name}")]
    ProfileNotFound { name: String },

    #[error("Audio encoding failed: {0}")]
    AudioEncode(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

impl From<pyo3::PyErr> for ChatterError {
    fn from(err: pyo3::PyErr) -> Self {
        Python::with_gil(|py| {
            let traceback = err.traceback(py)
                .map(|tb| tb.format().unwrap_or_default())
                .unwrap_or_default();
            ChatterError::PythonError {
                message: err.to_string(),
                traceback,
            }
        })
    }
}
```

### Pattern 4: Profile as TOML Metadata + Sidecar Audio

**What:** Each voice profile is a directory containing a `profile.toml` metadata file and optional audio files (reference audio for clones, cached sample for preview).

**When to use:** For all profile storage.

**Trade-offs:** Directory-per-profile is slightly more complex than a single JSON file but allows storing binary audio alongside metadata cleanly. TOML is human-readable and editable.

**Example structure:**
```
~/.config/chatter/profiles/
├── my-narrator/
│   ├── profile.toml          # metadata
│   ├── sample.mp3            # cached preview audio
│   └── reference.mp3         # original reference (clone profiles)
└── female-newscaster/
    ├── profile.toml
    └── sample.mp3
```

```toml
# profile.toml
name = "my-narrator"
type = "clone"                   # "clone" | "design" | "custom"
created = 2026-03-27T10:00:00Z
language = "English"
model_size = "1.7B"

[clone]
ref_text = "The quick brown fox jumps over the lazy dog."

[design]
instruct = ""                    # empty for clone profiles
```

## Data Flow

### Voice Design Flow

```
User: `chatter design --name warm-narrator --description "A warm male voice..."`
    |
    v
CLI Layer: parse args, validate
    |
    v
ProfileStore: check name not taken
    |
    v
InferenceEngine: Python::with_gil {
    |   1. ensure VoiceDesign model loaded (lazy)
    |   2. call model.generate_voice_design(
    |        text="sample text",
    |        language=language,
    |        instruct=description
    |      )
    |   3. return (wav_data, sample_rate)
    }
    |
    v
AudioPostProcessor: WAV -> MP3
    |
    v
ProfileStore: save profile.toml + sample.mp3
    |
    v
CLI Layer: print success, profile location
```

### Voice Clone Flow

```
User: `chatter clone --name my-voice --audio ref.mp3 --transcript "Hello world"`
    |
    v
CLI Layer: parse args, validate audio file exists
    |
    v
ProfileStore: check name not taken
    |
    v
InferenceEngine: Python::with_gil {
    |   1. ensure Base model loaded (lazy)
    |   2. call model.create_voice_clone_prompt(
    |        ref_audio=path, ref_text=transcript
    |      )
    |   3. call model.generate_voice_clone(
    |        text="sample text",
    |        language=language,
    |        voice_clone_prompt=prompt
    |      )
    |   4. return (wav_data, sample_rate, prompt_for_caching)
    }
    |
    v
AudioPostProcessor: WAV -> MP3
    |
    v
ProfileStore: save profile.toml + sample.mp3 + copy reference audio
    |
    v
CLI Layer: print success
```

### Speech Generation Flow

```
User: `chatter generate --profile my-voice --text "Hello world" --output hello.mp3`
    |
    v
CLI Layer: parse args, read input (text or file)
    |
    v
ProfileStore: load profile, determine model variant needed
    |
    v
InferenceEngine: Python::with_gil {
    |   1. ensure correct model variant loaded
    |   2. for clone profiles: rebuild voice_clone_prompt from stored ref audio
    |      for design profiles: use instruct from profile
    |      for custom profiles: use speaker name from profile
    |   3. call appropriate generate_* function
    |   4. relay progress via callback (if available)
    |   5. return (wav_data, sample_rate)
    }
    |
    v
AudioPostProcessor: WAV -> MP3
    |
    v
Write MP3 to output path
    |
    v
CLI Layer: print success, file path, duration
```

### Key Data Flows

1. **Model lifecycle:** Python interpreter initialized once in `main()`. Models loaded lazily on first use per variant. Model reference (`Py<PyAny>`) cached in `InferenceEngine` for process lifetime.
2. **GIL flow:** GIL acquired only during Python calls in `engine/`. Released immediately after. All Rust work (file I/O, MP3 encoding, profile management) happens outside the GIL.
3. **Audio pipeline:** Model produces numpy array -> converted to Vec<f32> via PyO3 -> encoded to MP3 in pure Rust -> written to disk.
4. **Error propagation:** Python exceptions caught at engine boundary -> converted to `ChatterError` -> rendered as user-friendly messages by CLI layer.

## Scaling Considerations

This is a local CLI tool, not a server. "Scaling" means handling large inputs.

| Concern | Approach |
|---------|----------|
| Long documents (PDF/TXT) | Chunk text into segments, generate per-chunk, concatenate audio |
| Multiple model variants | Load only the variant needed for current operation; different commands need different models |
| GPU memory | Only one model loaded at a time; unload if switching variants (or accept the memory cost for a short-lived CLI) |
| Large audio output | Stream WAV chunks to MP3 encoder incrementally rather than buffering entire output |

### First Bottleneck: Model Loading Time

Model loading is 10-30 seconds. For a CLI that runs a single command and exits, this is acceptable. If users frequently switch between operations needing different model variants, consider a daemon mode in the future (out of scope for v1).

### Second Bottleneck: Long Text Generation

For documents, text must be chunked. The qwen-tts API supports batch inference (list of strings), which should be used to process chunks efficiently in a single model call rather than looping.

## Anti-Patterns

### Anti-Pattern 1: Scattering PyO3 Calls Across Modules

**What people do:** Import `pyo3` in CLI commands, profile management, audio processing -- anywhere that "needs" Python.
**Why it's wrong:** Makes it impossible to reason about GIL lifetime, error boundaries, or test anything without a Python runtime. Couples entire codebase to PyO3.
**Do this instead:** Confine ALL `pyo3` usage to the `engine/` module. Everything else is pure Rust. The engine exposes a Rust-native API (no `PyObject` in its public interface).

### Anti-Pattern 2: Reloading Models Per Invocation

**What people do:** Call `from_pretrained()` inside every generate call because it "keeps things simple."
**Why it's wrong:** Model loading is the slowest operation (10-30s). Users will think the tool is broken.
**Do this instead:** Load once, cache in `Py<PyAny>`, reuse. The `InferenceEngine` struct owns the model lifetime.

### Anti-Pattern 3: Passing PyObject Through the Stack

**What people do:** Return `Py<PyAny>` from the engine and let callers extract data.
**Why it's wrong:** Leaks Python types beyond the boundary. Forces callers to acquire the GIL. Makes testing impossible without Python.
**Do this instead:** Convert all Python return values to Rust types (`Vec<f32>`, `u32` sample rate, etc.) inside the engine before returning.

### Anti-Pattern 4: Blocking on Python Without Progress Feedback

**What people do:** Call Python inference and wait silently for 30+ seconds.
**Why it's wrong:** Users think the tool is hung. They will Ctrl+C and file a bug.
**Do this instead:** At minimum, show a spinner. Ideally, show a progress bar with token count or percentage if the model API supports callbacks.

### Anti-Pattern 5: Storing Profiles as a Single JSON Database

**What people do:** Put all profiles in one `profiles.json` file.
**Why it's wrong:** Concurrent access issues, grows unwieldy, hard to include binary audio alongside metadata, harder to manually inspect/edit.
**Do this instead:** One directory per profile with TOML metadata + sidecar audio files.

## Integration Points

### External Services

| Service | Integration Pattern | Notes |
|---------|---------------------|-------|
| Hugging Face Hub | Python `qwen_tts` handles model download | First run downloads ~3-7GB model weights; cache in HF default location |
| CUDA/GPU | Via PyTorch in Python runtime | Must validate CUDA availability before attempting inference |

### Internal Boundaries

| Boundary | Communication | Notes |
|----------|---------------|-------|
| CLI <-> Engine | Rust function calls with Rust types | Engine returns `Result<AudioOutput, ChatterError>` -- no Python types leak |
| CLI <-> ProfileStore | Rust function calls with `Profile` structs | Pure Rust, serde-based |
| Engine <-> Python | PyO3 `with_gil` + `call_method` | GIL acquired/released per operation; model ref cached as `Py<PyAny>` |
| Engine <-> ProgressMgr | `Arc<ProgressBar>` shared via closure | Callback closure captures Arc, updates bar from within GIL |
| AudioProcessor <-> Engine | `AudioOutput { samples: Vec<f32>, sample_rate: u32 }` | Pure Rust struct, no Python dependency |

## Build Order (Dependencies Between Components)

The following order respects component dependencies and allows incremental validation:

1. **error.rs + profile/types.rs** -- Define error types and profile data structures. No dependencies. Everything else depends on these.
2. **profile/storage.rs** -- Profile CRUD on filesystem. Depends on types. Testable with no Python.
3. **engine/mod.rs + engine/models.rs** -- Python interpreter init, model loading. First PyO3 code. Validates that embedded Python works at all.
4. **engine/voice_design.rs** -- First inference pathway. Simplest API (no reference audio). Proves end-to-end Python bridge works.
5. **audio/encode.rs** -- WAV to MP3 conversion. Pure Rust. Can be developed in parallel with step 4.
6. **cli/design.rs** -- First complete command. Wires CLI -> Engine -> Audio -> Profile.
7. **engine/voice_clone.rs + cli/clone.rs** -- Second pathway. More complex (reference audio, prompt caching).
8. **engine/custom_voice.rs + cli/generate.rs** -- Third pathway. Requires profiles to exist.
9. **engine/progress.rs** -- Progress callback integration. Enhancement on top of working inference.
10. **cli/profiles.rs** -- Profile management commands. List, show, delete.
11. **Document/file input** -- PDF/TXT parsing, text chunking. Build last since it is an input preprocessing concern.

**Rationale:** The riskiest part is the PyO3 bridge (steps 3-4). Get that working early. Profile storage and audio encoding are pure Rust and can be developed in parallel. Progress feedback is a polish feature that should not block core functionality.

## Sources

- [PyO3 User Guide - Calling Python from Rust](https://pyo3.rs/v0.28.0/python-from-rust.html)
- [PyO3 GitHub Repository](https://github.com/PyO3/pyo3)
- [PyO3 `prepare_freethreaded_python` docs](https://docs.rs/pyo3/latest/pyo3/fn.prepare_freethreaded_python.html)
- [PyO3 GIL Discussion - `with_gil` and `Py<T>`](https://github.com/PyO3/pyo3/discussions/2255)
- [PyO3 `allow_threads` discussion](https://github.com/PyO3/pyo3/issues/640)
- [PyO3 Progress Callback Discussion](https://github.com/PyO3/pyo3/discussions/3659)
- [Qwen3-TTS GitHub - Full API](https://github.com/QwenLM/Qwen3-TTS)
- [qwen-tts PyPI package](https://pypi.org/project/qwen-tts/)
- [Combining Rust and Python for Automation (Jan 2026)](https://medium.com/@wim.henderickx/choice-matters-combining-rust-and-python-for-extensible-automation-systems-6534e3e7e2ed)

---
*Architecture research for: Rust CLI with embedded Python (PyO3) for TTS inference*
*Researched: 2026-03-27*
