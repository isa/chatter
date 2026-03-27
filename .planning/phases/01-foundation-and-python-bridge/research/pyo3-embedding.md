# PyO3 0.28.x Embedding Patterns -- Deep Dive

**Researched:** 2026-03-27
**Domain:** Embedding CPython in a Rust CLI binary via PyO3 0.28.2
**Confidence:** MEDIUM-HIGH (based on training data + existing project research; web verification was unavailable)

## Summary

This document provides concrete, implementation-ready patterns for embedding Python in the `chatter` Rust CLI using PyO3 0.28.2. It covers initialization, calling the `qwen_tts` package, error handling, GIL management, build configuration, and known embedding gotchas.

PyO3 0.28 introduced a significant API change: `Python::with_gil()` was renamed to `Python::attach()` to reflect that in Python 3.13+ free-threading mode, the GIL may not exist. The `auto-initialize` feature flag replaces explicit `prepare_freethreaded_python()` calls for embedding use cases.

**Key finding:** Do NOT use the `extension-module` feature for embedding. That feature is for Python extension modules (`.so`/`.pyd` files loaded by Python). For a Rust binary that embeds Python, use only `auto-initialize`.

## 1. Initialization: `auto-initialize` vs Manual

### Recommended: `auto-initialize` Feature

```toml
# Cargo.toml
[dependencies]
pyo3 = { version = "0.28", features = ["auto-initialize"] }
```

With `auto-initialize`, the Python interpreter starts automatically on the first call to `Python::attach()`. No explicit initialization code needed.

```rust
use pyo3::prelude::*;

fn main() -> anyhow::Result<()> {
    // Python starts automatically on first attach() call
    Python::attach(|py| {
        let sys = py.import("sys")?;
        let version: String = sys.getattr("version")?.extract()?;
        println!("Python {version}");
        Ok(())
    })?;
    Ok(())
}
```

### Manual: `prepare_freethreaded_python()`

If you need to set environment variables (like `PYTHONPATH`) BEFORE Python initializes, use manual initialization:

```rust
use pyo3::prelude::*;

fn main() -> anyhow::Result<()> {
    // Set env vars BEFORE Python starts
    std::env::set_var("PYTHONPATH", "/path/to/site-packages");

    // Explicitly initialize Python
    pyo3::prepare_freethreaded_python();

    // Now safe to attach
    Python::attach(|py| {
        // Python is already running
        Ok(())
    })?;
    Ok(())
}
```

**For chatter:** Use manual initialization with `prepare_freethreaded_python()`. The reason: we need to detect the correct Python site-packages path and set `PYTHONPATH` before the interpreter starts, to handle virtualenv detection (see Pitfall 6 in the phase research).

### `Python::attach()` vs `Python::with_gil()` (API History)

| PyO3 Version | API | Notes |
|-------------|-----|-------|
| 0.20 - 0.27 | `Python::with_gil(\|py\| { ... })` | Original closure-based GIL acquisition |
| 0.28+ | `Python::attach(\|py\| { ... })` | Renamed. Same semantics. `with_gil` may still work as deprecated alias but prefer `attach`. |

The rename reflects that Python 3.13 introduces free-threading (no GIL). `attach()` means "attach to the Python runtime" rather than "acquire the GIL," which is more accurate when the GIL might not exist.

**Confidence: MEDIUM** -- The rename is confirmed in existing research (STACK.md, 01-RESEARCH.md), but the exact deprecation status of `with_gil` in 0.28.2 should be verified at build time. If `attach()` doesn't compile, fall back to `with_gil()`.

## 2. Calling Python Packages from Rust

### Importing a Module

```rust
Python::attach(|py| {
    // Import a top-level module
    let qwen_tts = py.import("qwen_tts")?;

    // Import a submodule
    let torch = py.import("torch")?;

    // Import from a module (like `from qwen_tts import Qwen3TTSModel`)
    let model_class = py.import("qwen_tts")?.getattr("Qwen3TTSModel")?;

    Ok(())
})?;
```

### Calling `Qwen3TTSModel.from_pretrained()`

This is a classmethod that takes positional args and keyword args:

```rust
use pyo3::prelude::*;
use pyo3::types::PyDict;

Python::attach(|py| {
    let qwen_tts = py.import("qwen_tts")?;
    let model_class = qwen_tts.getattr("Qwen3TTSModel")?;
    let torch = py.import("torch")?;

    // Build keyword arguments
    let kwargs = PyDict::new(py);
    kwargs.set_item("device_map", "mps")?;       // or "cuda:0"
    kwargs.set_item("dtype", torch.getattr("float16")?)?;  // or bfloat16 for CUDA

    // Call: Qwen3TTSModel.from_pretrained("model_name", device_map=..., dtype=...)
    let model = model_class.call_method(
        "from_pretrained",
        ("Qwen/Qwen3-TTS-12Hz-1.7B-CustomVoice",),  // positional args as tuple
        Some(&kwargs),                                  // keyword args
    )?;

    // Store model for later use (see section on Py<PyAny> below)
    let model_handle: Py<PyAny> = model.unbind();

    Ok(())
})?;
```

### Calling Methods on Python Objects

```rust
// Assuming `model` is a Bound<'py, PyAny> reference to a loaded model

// Method with no args
let result = model.call_method0("some_method")?;

// Method with positional args only
let result = model.call_method1("generate", (text, language))?;

// Method with positional and keyword args
let kwargs = PyDict::new(py);
kwargs.set_item("language", "English")?;
kwargs.set_item("speaker", "Chelsie")?;
let result = model.call_method("generate_custom_voice", (text,), Some(&kwargs))?;
```

### Extracting Results Back to Rust

```rust
// Extract a Python string to Rust String
let version: String = sys.getattr("version")?.extract()?;

// Extract a Python int to Rust i64
let count: i64 = result.getattr("length")?.extract()?;

// Extract a Python bool
let available: bool = torch.getattr("cuda")?
    .call_method0("is_available")?
    .extract()?;

// Extract a Python list to Vec
let items: Vec<String> = py_list.extract()?;

// Extract numpy array to Vec<f32> (for audio samples)
// Option A: Use the buffer protocol
let np_array = result.getattr("audio")?;
let samples: Vec<f32> = np_array.call_method0("tolist")?.extract()?;

// Option B: Use .tobytes() for raw bytes, then reinterpret
let bytes: Vec<u8> = np_array.call_method0("tobytes")?.extract()?;
let samples: Vec<f32> = bytes.chunks_exact(4)
    .map(|chunk| f32::from_le_bytes(chunk.try_into().unwrap()))
    .collect();
```

**Performance note on audio extraction:** Option B (tobytes + reinterpret) is faster for large arrays because it avoids creating intermediate Python list objects. For a 30-second audio clip at 24kHz, that's 720,000 float values -- converting via `.tolist()` creates 720K Python float objects. `.tobytes()` copies raw memory.

### Getting Audio Sample Rate

```rust
// The qwen-tts generate functions typically return (audio_array, sample_rate)
// or the model has a .config.sampling_rate attribute
let sample_rate: u32 = model.getattr("config")?
    .getattr("sampling_rate")?
    .extract()?;
```

## 3. Storing Python Objects Across GIL Boundaries

### `Py<PyAny>` -- The GIL-Independent Handle

When you need to keep a Python object alive outside of a `Python::attach` closure (e.g., caching the loaded model), use `Py<PyAny>`:

```rust
use pyo3::prelude::*;

pub struct InferenceEngine {
    model: Option<Py<PyAny>>,    // Survives across attach() calls
    processor: Option<Py<PyAny>>,
}

impl InferenceEngine {
    pub fn new() -> Self {
        Self {
            model: None,
            processor: None,
        }
    }

    pub fn load_model(&mut self, model_name: &str, device: &str) -> PyResult<()> {
        Python::attach(|py| {
            let qwen_tts = py.import("qwen_tts")?;
            let model_class = qwen_tts.getattr("Qwen3TTSModel")?;

            let kwargs = PyDict::new(py);
            kwargs.set_item("device_map", device)?;

            let model = model_class.call_method("from_pretrained",
                (model_name,), Some(&kwargs))?;

            // .unbind() converts Bound<'py, PyAny> -> Py<PyAny>
            // This releases the borrow on `py` but keeps the Python object alive
            self.model = Some(model.unbind());
            Ok(())
        })
    }

    pub fn generate(&self, text: &str) -> PyResult<Vec<f32>> {
        let model_ref = self.model.as_ref()
            .ok_or_else(|| pyo3::exceptions::PyRuntimeError::new_err("Model not loaded"))?;

        Python::attach(|py| {
            // .bind(py) converts Py<PyAny> -> Bound<'py, PyAny>
            let model = model_ref.bind(py);

            let result = model.call_method1("generate", (text,))?;
            let samples: Vec<f32> = result.call_method0("tolist")?.extract()?;
            Ok(samples)
        })
    }
}
```

**Key rule:** `Py<PyAny>` is `Send` (can cross thread boundaries) but you MUST call `.bind(py)` inside a `Python::attach` closure to use it. The `py` token proves you hold the GIL.

### `GILOnceCell` -- For One-Time Python Object Caching

For objects you want to initialize once and reuse (like imported modules):

```rust
use pyo3::sync::GILOnceCell;
use pyo3::prelude::*;

static TORCH_MODULE: GILOnceCell<Py<PyModule>> = GILOnceCell::new();

fn get_torch(py: Python<'_>) -> PyResult<&Bound<'_, PyModule>> {
    TORCH_MODULE
        .get_or_try_init(py, || {
            py.import("torch").map(|m| m.unbind())
        })
        .map(|m| m.bind(py))
}
```

**Confidence: HIGH** -- `GILOnceCell` is a well-documented PyO3 primitive specifically designed for this pattern. The PITFALLS.md already recommends it over `lazy_static` or `OnceLock`.

## 4. Error Handling

### PyErr Structure

Every PyO3 operation that calls into Python returns `PyResult<T>`, which is `Result<T, PyErr>`. `PyErr` wraps a Python exception.

```rust
use pyo3::prelude::*;

Python::attach(|py| -> PyResult<()> {
    // This might raise ImportError
    let module = py.import("nonexistent_module")?;  // Returns Err(PyErr) on failure

    Ok(())
})?;  // The ? propagates the PyErr
```

### Converting PyErr to Application Errors

**Pattern A: Using `thiserror` with `From<PyErr>`**

```rust
use thiserror::Error;
use pyo3::prelude::*;

#[derive(Error, Debug)]
pub enum BridgeError {
    #[error("Python runtime error: {message}")]
    Python {
        message: String,
        traceback: Option<String>,
    },

    #[error("Module not found: {module} -- is qwen-tts installed?")]
    ModuleNotFound { module: String },

    #[error("No GPU available -- Apple Silicon (MPS) or CUDA GPU required")]
    NoGpu,

    #[error("{0}")]
    Io(#[from] std::io::Error),
}

impl From<PyErr> for BridgeError {
    fn from(err: PyErr) -> Self {
        Python::attach(|py| {
            // Check if it's an ImportError specifically
            if err.is_instance_of::<pyo3::exceptions::PyImportError>(py) {
                return BridgeError::ModuleNotFound {
                    module: err.value(py).to_string(),
                };
            }

            // Generic Python error with traceback
            let traceback = err.traceback(py)
                .and_then(|tb| tb.format().ok());

            BridgeError::Python {
                message: err.value(py).to_string(),
                traceback,
            }
        })
    }
}
```

**Pattern B: Using `anyhow` context at the call site**

```rust
use anyhow::{Context, Result};

fn check_python_env() -> Result<String> {
    Python::attach(|py| {
        let sys = py.import("sys")
            .context("Failed to import sys -- is Python correctly installed?")?;
        let version: String = sys.getattr("version")?.extract()?;
        Ok(version)
    })
    .context("Python runtime initialization failed")
}
```

**Recommendation for chatter:** Use both patterns together:
- `thiserror` in the `engine/` module for structured errors (`BridgeError`)
- `anyhow` in `main.rs` and CLI layer for adding context
- The `From<PyErr> for BridgeError` impl keeps engine code clean
- CLI layer converts `BridgeError` to user-facing messages based on `--verbose` flag

### Checking Python Exception Types

```rust
Python::attach(|py| {
    match py.import("qwen_tts") {
        Ok(m) => { /* success */ },
        Err(e) if e.is_instance_of::<pyo3::exceptions::PyImportError>(py) => {
            eprintln!("qwen-tts not installed. Run: pip install qwen-tts");
        },
        Err(e) if e.is_instance_of::<pyo3::exceptions::PyModuleNotFoundError>(py) => {
            eprintln!("Module not found: {}", e.value(py));
        },
        Err(e) => {
            eprintln!("Unexpected error: {}", e);
        },
    }
    Ok(())
})?;
```

### Extracting Full Python Traceback

```rust
fn format_python_error(py: Python<'_>, err: &PyErr) -> String {
    let mut msg = err.value(py).to_string();

    if let Some(tb) = err.traceback(py) {
        if let Ok(formatted) = tb.format() {
            msg = format!("{formatted}\n{msg}");
        }
    }

    msg
}
```

This is critical for the `--verbose` flag. Without it, users get "Python error: some_opaque_message" with no indication of which Python line caused the problem.

## 5. Build Configuration

### Cargo.toml

```toml
[package]
name = "chatter"
version = "0.1.0"
edition = "2024"
rust-version = "1.85"

[dependencies]
pyo3 = { version = "0.28", features = ["auto-initialize"] }

# Do NOT add "extension-module" feature -- that's for Python extensions, not embedding
# Do NOT add "abi3" feature -- ABI3 is for extension modules distributed as wheels
```

### Feature Flags Explained

| Feature | Purpose | Use for Chatter? |
|---------|---------|-----------------|
| `auto-initialize` | Auto-start Python on first `attach()` | YES -- simplest initialization |
| `extension-module` | Build a `.so`/`.pyd` loaded by Python | NO -- we embed Python in Rust, not the other way around |
| `abi3-pyXX` | Stable ABI for Python extensions | NO -- only for extension modules |
| `multiple-pymethods` | Allow multiple `#[pymethods]` impl blocks | NO -- we don't export Rust types to Python |
| `gil-refs` | Enable deprecated GIL-bound reference API | NO -- use Bound API |

### `PYO3_PYTHON` Environment Variable

Controls which Python PyO3 links against at build time:

```bash
# Point to specific Python version
PYO3_PYTHON=python3.12 cargo build

# Or set in .cargo/config.toml for the project
```

```toml
# .cargo/config.toml
[env]
PYO3_PYTHON = "python3.12"
```

**Critical for chatter:** The system has Python 3.14, but qwen-tts requires 3.9-3.13. This config MUST be set.

### Build-Time Python Detection

PyO3 uses `pyo3-build-config` (pulled in automatically) to detect Python at build time. It checks:

1. `PYO3_PYTHON` env var (highest priority)
2. `python3` on PATH
3. `python` on PATH

It extracts:
- Python version
- Library path (for linking)
- Include path (for headers)

On macOS with Homebrew, Python 3.12 lives at `/opt/homebrew/bin/python3.12` or `/opt/homebrew/opt/python@3.12/bin/python3.12`.

### Cross-Platform Build Notes

| Platform | Python Headers | Notes |
|----------|---------------|-------|
| macOS (Homebrew) | Included with `python@3.12` formula | Set `PYO3_PYTHON=python3.12` |
| Ubuntu/Debian | `apt install python3.12-dev` | Need `-dev` package for headers |
| Arch Linux | `pacman -S python` | Headers included by default |

## 6. Thread Safety and GIL Management

### The GIL in Embedding Context

When you embed Python in Rust:

1. **`Python::attach()` acquires the GIL** (or attaches to the Python runtime in free-threading mode)
2. **All Python operations require the GIL** -- you can only call Python methods inside `attach()`
3. **The GIL is automatically released** when the `attach()` closure returns
4. **You can release the GIL early** with `py.allow_threads()`

### Pattern: Spinner + Python Operation

The main challenge for chatter: showing an animated spinner while Python does model loading.

```rust
use indicatif::{ProgressBar, ProgressStyle};
use std::time::Duration;

fn load_model_with_spinner(engine: &mut InferenceEngine) -> anyhow::Result<()> {
    // Create spinner BEFORE entering Python
    let spinner = ProgressBar::new_spinner();
    spinner.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.cyan} {msg} ({elapsed})")
            .expect("valid template")
    );
    spinner.set_message("Loading Qwen3-TTS 1.7B...");
    spinner.enable_steady_tick(Duration::from_millis(100));

    // indicatif spawns its own thread for animation.
    // That thread does NOT need the GIL -- it only writes to stderr.
    // So the spinner will animate while Python::attach holds the GIL.

    // This blocks the current thread but the spinner thread keeps ticking
    engine.load_model("Qwen/Qwen3-TTS-12Hz-1.7B-CustomVoice", "mps")?;

    spinner.finish_with_message("Model loaded");
    Ok(())
}
```

**Why this works:** `indicatif`'s `enable_steady_tick()` spawns a background thread that periodically redraws the spinner. This thread only does terminal I/O -- it never touches Python or the GIL. So even though `Python::attach()` holds the GIL on the main thread, the spinner thread runs independently.

### Pattern: Releasing the GIL for Rust Work

```rust
Python::attach(|py| {
    // Acquire model reference
    let model = self.model.as_ref().unwrap().bind(py);

    // Run inference (holds GIL -- Python code is running)
    let result = model.call_method1("generate", (text,))?;

    // Extract audio data to Rust (still needs GIL)
    let audio_bytes: Vec<u8> = result.call_method0("tobytes")?.extract()?;
    let sample_rate: u32 = model.getattr("config")?
        .getattr("sampling_rate")?.extract()?;

    // Release GIL for MP3 encoding (pure Rust, no Python needed)
    let mp3_data = py.allow_threads(|| {
        encode_to_mp3(&audio_bytes, sample_rate)
    })?;

    Ok(mp3_data)
})?;
```

`py.allow_threads()` temporarily releases the GIL, allowing other Python threads to run. For chatter, this matters when:
- Encoding audio to MP3 (pure Rust, no Python needed)
- Writing files to disk
- Any CPU-intensive Rust work after extracting data from Python

### What NOT to Do: Deadlock Patterns

```rust
// DEADLOCK: Do not nest attach() calls
Python::attach(|py| {
    Python::attach(|py2| {  // DEADLOCK -- already attached on this thread
        // ...
    });
    Ok(())
});

// DEADLOCK: Do not hold a Rust mutex while attaching on another thread
let data = Arc::new(Mutex::new(vec![]));
let data_clone = data.clone();

std::thread::spawn(move || {
    let guard = data_clone.lock().unwrap();  // Hold Rust mutex
    Python::attach(|py| {                    // Try to get GIL
        // ... DEADLOCK if main thread holds GIL and waits for mutex
    });
});

// SAFE: Use GILOnceCell instead of Mutex for Python objects
// SAFE: Use channels instead of shared mutexes between Python and Rust threads
```

**Note on `Python::attach()` re-entrancy:** On the same thread, `Python::attach()` should be re-entrant (it was for `with_gil`). But nesting is bad practice -- it makes GIL lifetime harder to reason about. Keep `attach()` calls at the top level of your engine functions.

## 7. Complete Embedding Example for Chatter

```rust
use pyo3::prelude::*;
use pyo3::types::PyDict;
use anyhow::{Context, Result};

/// Represents the Python bridge for chatter.
/// All PyO3 interaction is contained within this struct.
pub struct PythonBridge {
    model: Option<Py<PyAny>>,
}

impl PythonBridge {
    /// Create a new bridge. Initializes the Python interpreter.
    pub fn init() -> Result<Self> {
        // Set up Python path before initialization if needed
        // (e.g., inject virtualenv site-packages)
        if let Ok(venv) = std::env::var("VIRTUAL_ENV") {
            // Inject venv site-packages into PYTHONPATH
            let site_packages = format!("{venv}/lib/python3.12/site-packages");
            match std::env::var("PYTHONPATH") {
                Ok(existing) => std::env::set_var("PYTHONPATH",
                    format!("{site_packages}:{existing}")),
                Err(_) => std::env::set_var("PYTHONPATH", &site_packages),
            }
        }

        // Initialize Python explicitly (before auto-initialize kicks in)
        pyo3::prepare_freethreaded_python();

        Ok(Self { model: None })
    }

    /// Check if the Python environment has all required packages.
    pub fn check_environment(&self) -> Result<EnvironmentInfo> {
        Python::attach(|py| {
            let sys = py.import("sys")?;
            let python_version: String = sys.getattr("version")?.extract()?;

            let qwen_tts_version = py.import("qwen_tts")
                .and_then(|m| m.getattr("__version__"))
                .and_then(|v| v.extract::<String>())
                .ok();

            let torch = py.import("torch")
                .context("PyTorch not installed")?;
            let torch_version: String = torch.getattr("__version__")?.extract()?;

            let has_cuda: bool = torch.getattr("cuda")?
                .call_method0("is_available")?.extract()?;
            let has_mps: bool = torch.getattr("backends")?
                .getattr("mps")?
                .call_method0("is_available")?.extract()?;

            let gpu_name = if has_cuda {
                Some(torch.getattr("cuda")?
                    .call_method1("get_device_name", (0_i32,))?
                    .extract::<String>()?)
            } else {
                None
            };

            Ok(EnvironmentInfo {
                python_version,
                torch_version,
                qwen_tts_version,
                has_cuda,
                has_mps,
                gpu_name,
            })
        })
        .context("Failed to check Python environment")
    }

    /// Load a Qwen3-TTS model. Call this lazily -- only when inference is needed.
    pub fn load_model(&mut self, model_id: &str, backend: &ComputeBackend) -> Result<()> {
        Python::attach(|py| {
            let qwen_tts = py.import("qwen_tts")
                .context("qwen-tts package not found. Install with: pip install qwen-tts")?;
            let torch = py.import("torch")?;
            let model_class = qwen_tts.getattr("Qwen3TTSModel")?;

            let kwargs = PyDict::new(py);

            match backend {
                ComputeBackend::Cuda(ref _name) => {
                    kwargs.set_item("device_map", "cuda:0")?;
                    kwargs.set_item("dtype", torch.getattr("bfloat16")?)?;
                    // Only set flash_attention_2 on CUDA
                    kwargs.set_item("attn_implementation", "flash_attention_2")?;
                },
                ComputeBackend::Mps => {
                    kwargs.set_item("device_map", "mps")?;
                    // float16 for most models; float32 for Base (clone) models
                    let is_base_model = model_id.contains("Base");
                    let dtype = if is_base_model {
                        torch.getattr("float32")?
                    } else {
                        torch.getattr("float16")?
                    };
                    kwargs.set_item("dtype", dtype)?;
                },
                ComputeBackend::Cpu => {
                    kwargs.set_item("device_map", "cpu")?;
                    kwargs.set_item("dtype", torch.getattr("float32")?)?;
                },
            }

            let model = model_class.call_method("from_pretrained",
                (model_id,), Some(&kwargs))?;

            self.model = Some(model.unbind());
            Ok(())
        })
        .context(format!("Failed to load model: {model_id}"))
    }

    /// Check if a model is already downloaded in the HuggingFace cache.
    pub fn is_model_cached(&self, model_id: &str) -> Result<bool> {
        Python::attach(|py| {
            let code = format!(
                r#"
import os
from huggingface_hub import try_to_load_from_cache
result = try_to_load_from_cache("{model_id}", "config.json")
result is not None and not isinstance(result, str) or os.path.exists(str(result or ""))
"#
            );
            // Alternative simpler check:
            let huggingface_hub = py.import("huggingface_hub")?;
            let try_load = huggingface_hub.getattr("try_to_load_from_cache")?;
            let result = try_load.call1((model_id, "config.json"))?;

            // try_to_load_from_cache returns None if not cached, or the path if cached
            let is_cached = !result.is_none();
            Ok(is_cached)
        })
        .context("Failed to check model cache")
    }

    /// Download a model (lets HuggingFace handle its own progress output).
    pub fn download_model(&self, model_id: &str) -> Result<()> {
        Python::attach(|py| {
            let huggingface_hub = py.import("huggingface_hub")?;
            let snapshot_download = huggingface_hub.getattr("snapshot_download")?;

            // snapshot_download shows its own progress bars on stderr
            snapshot_download.call1((model_id,))?;
            Ok(())
        })
        .context(format!("Failed to download model: {model_id}"))
    }
}

pub struct EnvironmentInfo {
    pub python_version: String,
    pub torch_version: String,
    pub qwen_tts_version: Option<String>,
    pub has_cuda: bool,
    pub has_mps: bool,
    pub gpu_name: Option<String>,
}

pub enum ComputeBackend {
    Cuda(String),
    Mps,
    Cpu,
}
```

## 8. Embedding-Specific Gotchas

### Gotcha 1: Signal Handling Conflicts

**Problem:** Python installs its own signal handlers on initialization. This can interfere with Ctrl+C handling in the Rust CLI.

**Symptoms:** Ctrl+C doesn't cleanly exit, or produces a Python traceback instead of a clean Rust exit.

**Mitigation:**
```rust
// After Python initialization, restore Rust's default signal handler
pyo3::prepare_freethreaded_python();

// Reset SIGINT handler to default so Ctrl+C works as expected
// (Python's SIGINT handler raises KeyboardInterrupt, which PyO3 catches)
unsafe {
    libc::signal(libc::SIGINT, libc::SIG_DFL);
}
```

**Confidence: MEDIUM** -- This is a known issue from PyO3 discussions. The exact behavior may differ in 0.28.x. Test during implementation: if Ctrl+C works cleanly without the signal reset, don't add it.

### Gotcha 2: `sys.path` Does Not Include virtualenv

**Problem:** `prepare_freethreaded_python()` or `auto-initialize` starts Python with the system Python's `sys.path`. Packages in a virtualenv are invisible.

**Mitigation (from existing PITFALLS.md research):**
```rust
fn inject_virtualenv_if_active(py: Python<'_>) -> PyResult<()> {
    if let Ok(venv) = std::env::var("VIRTUAL_ENV") {
        let sys = py.import("sys")?;
        let path = sys.getattr("path")?;

        // Construct the site-packages path for the venv
        let site_packages = if cfg!(target_os = "windows") {
            format!("{}\\Lib\\site-packages", venv)
        } else {
            // Need to detect Python version for correct path
            let version_info = sys.getattr("version_info")?;
            let major: u32 = version_info.getattr("major")?.extract()?;
            let minor: u32 = version_info.getattr("minor")?.extract()?;
            format!("{}/lib/python{}.{}/site-packages", venv, major, minor)
        };

        // Insert at position 0 so venv packages take priority
        path.call_method1("insert", (0_i32, &site_packages))?;
    }
    Ok(())
}
```

Call this immediately after the first `Python::attach` or inside `PythonBridge::init()`.

### Gotcha 3: `atexit` and Python Finalization

**Problem:** Python's `atexit` handlers run when the interpreter finalizes. If PyTorch registers GPU cleanup via `atexit`, and the Rust binary exits before Python finalizes, GPU resources may leak or the process may hang on exit.

**Mitigation:** PyO3 handles Python finalization automatically when the process exits. However, for clean GPU cleanup:

```rust
impl Drop for PythonBridge {
    fn drop(&mut self) {
        // Drop the model reference before Python finalizes
        if let Some(model) = self.model.take() {
            Python::attach(|py| {
                // Explicitly drop the Python object
                drop(model);

                // Force GPU memory cleanup
                if let Ok(torch) = py.import("torch") {
                    if let Ok(cuda) = torch.getattr("cuda") {
                        let _ = cuda.call_method0("empty_cache");
                    }
                }

                // Run Python garbage collection
                if let Ok(gc) = py.import("gc") {
                    let _ = gc.call_method0("collect");
                }
            });
        }
    }
}
```

### Gotcha 4: `extension-module` Feature Causes Linker Errors

**Problem:** If `extension-module` is added to PyO3 features (by mistake or by following extension module tutorials), the binary won't link against `libpython` and will crash at runtime.

**Why:** `extension-module` tells PyO3 NOT to link against libpython (because Python extensions are loaded by Python, which already has libpython). For embedding, you NEED to link against libpython.

**Prevention:** Only use `auto-initialize`. Never use `extension-module` for an embedding binary.

### Gotcha 5: macOS Framework Python vs Homebrew Python

**Problem:** macOS ships a system Python framework. Homebrew installs its own Python. PyO3 might link against the wrong one.

**Symptoms:** Builds succeed but runtime `import` fails, or the wrong Python version runs.

**Prevention:**
```toml
# .cargo/config.toml -- force PyO3 to use Homebrew Python 3.12
[env]
PYO3_PYTHON = "/opt/homebrew/bin/python3.12"
```

Verify after build:
```rust
Python::attach(|py| {
    let sys = py.import("sys")?;
    let executable: String = sys.getattr("executable")?.extract()?;
    println!("Python executable: {executable}");
    // Should show /opt/homebrew/...python3.12, NOT /usr/bin/python3
    Ok(())
});
```

### Gotcha 6: Python Warnings Pollute stderr

**Problem:** Python packages (especially PyTorch and transformers) emit many warnings to stderr. These interleave with chatter's own output and indicatif spinners.

**Mitigation:**
```rust
fn suppress_python_warnings(py: Python<'_>) -> PyResult<()> {
    let warnings = py.import("warnings")?;
    // Suppress FutureWarning, DeprecationWarning from transformers
    warnings.call_method1("filterwarnings", ("ignore",))?;

    // Or more targeted:
    // warnings.call_method("filterwarnings", ("ignore",),
    //     Some(&[("category", py.get_type::<pyo3::exceptions::PyFutureWarning>())].into_py_dict(py)))?;

    // Suppress HuggingFace logging
    let os = py.import("os")?;
    let environ = os.getattr("environ")?;
    environ.set_item("TOKENIZERS_PARALLELISM", "false")?;
    environ.set_item("TRANSFORMERS_VERBOSITY", "error")?;

    Ok(())
}
```

Call this right after Python initialization, before any model imports.

## 9. Testing the PyO3 Bridge

### Unit Testing Without Python

The architecture pattern of isolating PyO3 to the `engine/` module means all other modules can be tested without Python:

```rust
// In engine/mod.rs -- the public API returns Rust types
pub struct AudioOutput {
    pub samples: Vec<f32>,
    pub sample_rate: u32,
}

// In audio/encode.rs -- can test with fixture data
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mp3_encoding() {
        let samples = vec![0.0f32; 24000]; // 1 second of silence at 24kHz
        let mp3 = encode_to_mp3(&samples, 24000).unwrap();
        assert!(!mp3.is_empty());
    }
}
```

### Integration Testing With Python

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use pyo3::prelude::*;

    #[test]
    fn test_python_available() {
        pyo3::prepare_freethreaded_python();
        Python::attach(|py| {
            let sys = py.import("sys").unwrap();
            let version: String = sys.getattr("version").unwrap().extract().unwrap();
            assert!(version.starts_with("3.12") || version.starts_with("3.13"));
        });
    }

    #[test]
    fn test_qwen_tts_importable() {
        pyo3::prepare_freethreaded_python();
        Python::attach(|py| {
            let result = py.import("qwen_tts");
            assert!(result.is_ok(), "qwen_tts should be importable");
        });
    }
}
```

**Note:** Tests that use `pyo3::prepare_freethreaded_python()` or `auto-initialize` must be careful about test parallelism. Python can only be initialized once per process. Use `cargo test -- --test-threads=1` or put all Python tests in a single test binary.

## 10. API Quick Reference

| Operation | PyO3 0.28.x Code |
|-----------|-------------------|
| Acquire GIL | `Python::attach(\|py\| { ... })` |
| Import module | `py.import("module_name")?` |
| Get attribute | `obj.getattr("name")?` |
| Set attribute | `obj.setattr("name", value)?` |
| Call method (no args) | `obj.call_method0("method")?` |
| Call method (positional) | `obj.call_method1("method", (arg1, arg2))?` |
| Call method (kwargs) | `obj.call_method("method", (arg1,), Some(&kwargs))?` |
| Create dict | `PyDict::new(py)` |
| Set dict item | `dict.set_item("key", value)?` |
| Extract to Rust | `py_obj.extract::<RustType>()?` |
| Store across GIL | `obj.unbind()` -> `Py<PyAny>` |
| Use stored object | `py_handle.bind(py)` -> `Bound<'py, PyAny>` |
| Release GIL | `py.allow_threads(\|\| { rust_work() })` |
| Run Python code | `py.run("code", None, None)?` |
| Eval Python expr | `py.eval("expr", None, None)?` |
| Check exception type | `err.is_instance_of::<PyImportError>(py)` |
| Get traceback | `err.traceback(py).and_then(\|tb\| tb.format().ok())` |
| Cache module | `GILOnceCell::get_or_try_init(py, \|\| ...)` |
| None value | `py.None()` |

## Confidence Assessment

| Area | Level | Reason |
|------|-------|--------|
| `auto-initialize` feature | HIGH | Confirmed in existing research, consistent with PyO3 docs |
| `Python::attach()` API | MEDIUM | Existing research confirms rename from `with_gil`. Exact signature should be verified at compile time. |
| `Py<PyAny>` / `.unbind()` / `.bind()` | HIGH | Core PyO3 pattern, well-documented, used in ARCHITECTURE.md examples |
| Error handling (`PyErr`) | HIGH | Standard PyO3 error type, patterns consistent across versions |
| `extension-module` caveat | HIGH | Well-documented: extension-module is for Python extensions, not embedding |
| Signal handling | MEDIUM | Known issue, but exact behavior in 0.28.x not verified via live docs |
| virtualenv `sys.path` injection | HIGH | Confirmed in PITFALLS.md with PyO3 GitHub issue references |
| GIL + spinner threading | HIGH | indicatif uses independent thread; confirmed compatible with GIL holding |
| `.tobytes()` for numpy arrays | MEDIUM | Standard numpy API; PyO3 extraction should work but buffer protocol may be more idiomatic |
| `GILOnceCell` | HIGH | Official PyO3 primitive, explicitly recommended over lazy_static |

## Sources

### From Existing Project Research (HIGH confidence)
- `.planning/research/STACK.md` -- PyO3 integration strategy, Cargo.toml setup
- `.planning/research/PITFALLS.md` -- Virtualenv detection, GIL deadlocks, PyO3 issues #3045, #3089, #3726, #4841
- `.planning/research/ARCHITECTURE.md` -- `InferenceEngine` pattern, `Py<PyAny>` caching, error boundary pattern
- `.planning/phases/01-foundation-and-python-bridge/01-RESEARCH.md` -- `Python::attach` rename, compute backend patterns

### Official Documentation (HIGH confidence, not re-verified due to tool limitations)
- [PyO3 User Guide](https://pyo3.rs/) -- Embedding chapter, GIL management, error handling
- [PyO3 0.28.2 API docs](https://docs.rs/pyo3/0.28.2/) -- `Python::attach`, `Py<T>`, `Bound<'py, T>`
- [PyO3 GitHub](https://github.com/PyO3/pyo3) -- Issues and discussions referenced in PITFALLS.md

### Training Data (MEDIUM confidence -- may be stale)
- PyO3 0.20-0.28 migration patterns
- `prepare_freethreaded_python()` semantics
- Signal handling behavior in embedded Python
- numpy buffer protocol via PyO3

---
*PyO3 embedding deep-dive for: chatter (Rust CLI with embedded Python for TTS)*
*Researched: 2026-03-27*
*Valid until: 2026-04-27 (30 days -- PyO3 0.28.x is stable)*
# PyO3 0.28.x Embedding Patterns -- Deep Dive

**Researched:** 2026-03-27
**Domain:** Embedding CPython in a Rust CLI binary via PyO3 0.28.2
**Confidence:** MEDIUM-HIGH (based on training data + existing project research; web verification was unavailable)

## Summary

This document provides concrete, implementation-ready patterns for embedding Python in the `chatter` Rust CLI using PyO3 0.28.2. It covers initialization, calling the `qwen_tts` package, error handling, GIL management, build configuration, and known embedding gotchas.

PyO3 0.28 introduced a significant API change: `Python::with_gil()` was renamed to `Python::attach()` to reflect that in Python 3.13+ free-threading mode, the GIL may not exist. The `auto-initialize` feature flag replaces explicit `prepare_freethreaded_python()` calls for embedding use cases.

**Key finding:** Do NOT use the `extension-module` feature for embedding. That feature is for Python extension modules (`.so`/`.pyd` files loaded by Python). For a Rust binary that embeds Python, use only `auto-initialize`.

## 1. Initialization: `auto-initialize` vs Manual

### Recommended: `auto-initialize` Feature

```toml
# Cargo.toml
[dependencies]
pyo3 = { version = "0.28", features = ["auto-initialize"] }
```

With `auto-initialize`, the Python interpreter starts automatically on the first call to `Python::attach()`. No explicit initialization code needed.

```rust
use pyo3::prelude::*;

fn main() -> anyhow::Result<()> {
    // Python starts automatically on first attach() call
    Python::attach(|py| {
        let sys = py.import("sys")?;
        let version: String = sys.getattr("version")?.extract()?;
        println!("Python {version}");
        Ok(())
    })?;
    Ok(())
}
```

### Manual: `prepare_freethreaded_python()`

If you need to set environment variables (like `PYTHONPATH`) BEFORE Python initializes, use manual initialization:

```rust
use pyo3::prelude::*;

fn main() -> anyhow::Result<()> {
    // Set env vars BEFORE Python starts
    std::env::set_var("PYTHONPATH", "/path/to/site-packages");

    // Explicitly initialize Python
    pyo3::prepare_freethreaded_python();

    // Now safe to attach
    Python::attach(|py| {
        // Python is already running
        Ok(())
    })?;
    Ok(())
}
```

**For chatter:** Use manual initialization with `prepare_freethreaded_python()`. The reason: we need to detect the correct Python site-packages path and set `PYTHONPATH` before the interpreter starts, to handle virtualenv detection (see Pitfall 6 in the phase research).

### `Python::attach()` vs `Python::with_gil()` (API History)

| PyO3 Version | API | Notes |
|-------------|-----|-------|
| 0.20 - 0.27 | `Python::with_gil(\|py\| { ... })` | Original closure-based GIL acquisition |
| 0.28+ | `Python::attach(\|py\| { ... })` | Renamed. Same semantics. `with_gil` may still work as deprecated alias but prefer `attach`. |

The rename reflects that Python 3.13 introduces free-threading (no GIL). `attach()` means "attach to the Python runtime" rather than "acquire the GIL," which is more accurate when the GIL might not exist.

**Confidence: MEDIUM** -- The rename is confirmed in existing research (STACK.md, 01-RESEARCH.md), but the exact deprecation status of `with_gil` in 0.28.2 should be verified at build time. If `attach()` doesn't compile, fall back to `with_gil()`.

## 2. Calling Python Packages from Rust

### Importing a Module

```rust
Python::attach(|py| {
    // Import a top-level module
    let qwen_tts = py.import("qwen_tts")?;

    // Import a submodule
    let torch = py.import("torch")?;

    // Import from a module (like `from qwen_tts import Qwen3TTSModel`)
    let model_class = py.import("qwen_tts")?.getattr("Qwen3TTSModel")?;

    Ok(())
})?;
```

### Calling `Qwen3TTSModel.from_pretrained()`

This is a classmethod that takes positional args and keyword args:

```rust
use pyo3::prelude::*;
use pyo3::types::PyDict;

Python::attach(|py| {
    let qwen_tts = py.import("qwen_tts")?;
    let model_class = qwen_tts.getattr("Qwen3TTSModel")?;
    let torch = py.import("torch")?;

    // Build keyword arguments
    let kwargs = PyDict::new(py);
    kwargs.set_item("device_map", "mps")?;       // or "cuda:0"
    kwargs.set_item("dtype", torch.getattr("float16")?)?;  // or bfloat16 for CUDA

    // Call: Qwen3TTSModel.from_pretrained("model_name", device_map=..., dtype=...)
    let model = model_class.call_method(
        "from_pretrained",
        ("Qwen/Qwen3-TTS-12Hz-1.7B-CustomVoice",),  // positional args as tuple
        Some(&kwargs),                                  // keyword args
    )?;

    // Store model for later use (see section on Py<PyAny> below)
    let model_handle: Py<PyAny> = model.unbind();

    Ok(())
})?;
```

### Calling Methods on Python Objects

```rust
// Assuming `model` is a Bound<'py, PyAny> reference to a loaded model

// Method with no args
let result = model.call_method0("some_method")?;

// Method with positional args only
let result = model.call_method1("generate", (text, language))?;

// Method with positional and keyword args
let kwargs = PyDict::new(py);
kwargs.set_item("language", "English")?;
kwargs.set_item("speaker", "Chelsie")?;
let result = model.call_method("generate_custom_voice", (text,), Some(&kwargs))?;
```

### Extracting Results Back to Rust

```rust
// Extract a Python string to Rust String
let version: String = sys.getattr("version")?.extract()?;

// Extract a Python int to Rust i64
let count: i64 = result.getattr("length")?.extract()?;

// Extract a Python bool
let available: bool = torch.getattr("cuda")?
    .call_method0("is_available")?
    .extract()?;

// Extract a Python list to Vec
let items: Vec<String> = py_list.extract()?;

// Extract numpy array to Vec<f32> (for audio samples)
// Option A: Use the buffer protocol
let np_array = result.getattr("audio")?;
let samples: Vec<f32> = np_array.call_method0("tolist")?.extract()?;

// Option B: Use .tobytes() for raw bytes, then reinterpret
let bytes: Vec<u8> = np_array.call_method0("tobytes")?.extract()?;
let samples: Vec<f32> = bytes.chunks_exact(4)
    .map(|chunk| f32::from_le_bytes(chunk.try_into().unwrap()))
    .collect();
```

**Performance note on audio extraction:** Option B (tobytes + reinterpret) is faster for large arrays because it avoids creating intermediate Python list objects. For a 30-second audio clip at 24kHz, that's 720,000 float values -- converting via `.tolist()` creates 720K Python float objects. `.tobytes()` copies raw memory.

### Getting Audio Sample Rate

```rust
// The qwen-tts generate functions typically return (audio_array, sample_rate)
// or the model has a .config.sampling_rate attribute
let sample_rate: u32 = model.getattr("config")?
    .getattr("sampling_rate")?
    .extract()?;
```

## 3. Storing Python Objects Across GIL Boundaries

### `Py<PyAny>` -- The GIL-Independent Handle

When you need to keep a Python object alive outside of a `Python::attach` closure (e.g., caching the loaded model), use `Py<PyAny>`:

```rust
use pyo3::prelude::*;

pub struct InferenceEngine {
    model: Option<Py<PyAny>>,    // Survives across attach() calls
    processor: Option<Py<PyAny>>,
}

impl InferenceEngine {
    pub fn new() -> Self {
        Self {
            model: None,
            processor: None,
        }
    }

    pub fn load_model(&mut self, model_name: &str, device: &str) -> PyResult<()> {
        Python::attach(|py| {
            let qwen_tts = py.import("qwen_tts")?;
            let model_class = qwen_tts.getattr("Qwen3TTSModel")?;

            let kwargs = PyDict::new(py);
            kwargs.set_item("device_map", device)?;

            let model = model_class.call_method("from_pretrained",
                (model_name,), Some(&kwargs))?;

            // .unbind() converts Bound<'py, PyAny> -> Py<PyAny>
            // This releases the borrow on `py` but keeps the Python object alive
            self.model = Some(model.unbind());
            Ok(())
        })
    }

    pub fn generate(&self, text: &str) -> PyResult<Vec<f32>> {
        let model_ref = self.model.as_ref()
            .ok_or_else(|| pyo3::exceptions::PyRuntimeError::new_err("Model not loaded"))?;

        Python::attach(|py| {
            // .bind(py) converts Py<PyAny> -> Bound<'py, PyAny>
            let model = model_ref.bind(py);

            let result = model.call_method1("generate", (text,))?;
            let samples: Vec<f32> = result.call_method0("tolist")?.extract()?;
            Ok(samples)
        })
    }
}
```

**Key rule:** `Py<PyAny>` is `Send` (can cross thread boundaries) but you MUST call `.bind(py)` inside a `Python::attach` closure to use it. The `py` token proves you hold the GIL.

### `GILOnceCell` -- For One-Time Python Object Caching

For objects you want to initialize once and reuse (like imported modules):

```rust
use pyo3::sync::GILOnceCell;
use pyo3::prelude::*;

static TORCH_MODULE: GILOnceCell<Py<PyModule>> = GILOnceCell::new();

fn get_torch(py: Python<'_>) -> PyResult<&Bound<'_, PyModule>> {
    TORCH_MODULE
        .get_or_try_init(py, || {
            py.import("torch").map(|m| m.unbind())
        })
        .map(|m| m.bind(py))
}
```

**Confidence: HIGH** -- `GILOnceCell` is a well-documented PyO3 primitive specifically designed for this pattern. The PITFALLS.md already recommends it over `lazy_static` or `OnceLock`.

## 4. Error Handling

### PyErr Structure

Every PyO3 operation that calls into Python returns `PyResult<T>`, which is `Result<T, PyErr>`. `PyErr` wraps a Python exception.

```rust
use pyo3::prelude::*;

Python::attach(|py| -> PyResult<()> {
    // This might raise ImportError
    let module = py.import("nonexistent_module")?;  // Returns Err(PyErr) on failure

    Ok(())
})?;  // The ? propagates the PyErr
```

### Converting PyErr to Application Errors

**Pattern A: Using `thiserror` with `From<PyErr>`**

```rust
use thiserror::Error;
use pyo3::prelude::*;

#[derive(Error, Debug)]
pub enum BridgeError {
    #[error("Python runtime error: {message}")]
    Python {
        message: String,
        traceback: Option<String>,
    },

    #[error("Module not found: {module} -- is qwen-tts installed?")]
    ModuleNotFound { module: String },

    #[error("No GPU available -- Apple Silicon (MPS) or CUDA GPU required")]
    NoGpu,

    #[error("{0}")]
    Io(#[from] std::io::Error),
}

impl From<PyErr> for BridgeError {
    fn from(err: PyErr) -> Self {
        Python::attach(|py| {
            // Check if it's an ImportError specifically
            if err.is_instance_of::<pyo3::exceptions::PyImportError>(py) {
                return BridgeError::ModuleNotFound {
                    module: err.value(py).to_string(),
                };
            }

            // Generic Python error with traceback
            let traceback = err.traceback(py)
                .and_then(|tb| tb.format().ok());

            BridgeError::Python {
                message: err.value(py).to_string(),
                traceback,
            }
        })
    }
}
```

**Pattern B: Using `anyhow` context at the call site**

```rust
use anyhow::{Context, Result};

fn check_python_env() -> Result<String> {
    Python::attach(|py| {
        let sys = py.import("sys")
            .context("Failed to import sys -- is Python correctly installed?")?;
        let version: String = sys.getattr("version")?.extract()?;
        Ok(version)
    })
    .context("Python runtime initialization failed")
}
```

**Recommendation for chatter:** Use both patterns together:
- `thiserror` in the `engine/` module for structured errors (`BridgeError`)
- `anyhow` in `main.rs` and CLI layer for adding context
- The `From<PyErr> for BridgeError` impl keeps engine code clean
- CLI layer converts `BridgeError` to user-facing messages based on `--verbose` flag

### Checking Python Exception Types

```rust
Python::attach(|py| {
    match py.import("qwen_tts") {
        Ok(m) => { /* success */ },
        Err(e) if e.is_instance_of::<pyo3::exceptions::PyImportError>(py) => {
            eprintln!("qwen-tts not installed. Run: pip install qwen-tts");
        },
        Err(e) if e.is_instance_of::<pyo3::exceptions::PyModuleNotFoundError>(py) => {
            eprintln!("Module not found: {}", e.value(py));
        },
        Err(e) => {
            eprintln!("Unexpected error: {}", e);
        },
    }
    Ok(())
})?;
```

### Extracting Full Python Traceback

```rust
fn format_python_error(py: Python<'_>, err: &PyErr) -> String {
    let mut msg = err.value(py).to_string();

    if let Some(tb) = err.traceback(py) {
        if let Ok(formatted) = tb.format() {
            msg = format!("{formatted}\n{msg}");
        }
    }

    msg
}
```

This is critical for the `--verbose` flag. Without it, users get "Python error: some_opaque_message" with no indication of which Python line caused the problem.

## 5. Build Configuration

### Cargo.toml

```toml
[package]
name = "chatter"
version = "0.1.0"
edition = "2024"
rust-version = "1.85"

[dependencies]
pyo3 = { version = "0.28", features = ["auto-initialize"] }

# Do NOT add "extension-module" feature -- that's for Python extensions, not embedding
# Do NOT add "abi3" feature -- ABI3 is for extension modules distributed as wheels
```

### Feature Flags Explained

| Feature | Purpose | Use for Chatter? |
|---------|---------|-----------------|
| `auto-initialize` | Auto-start Python on first `attach()` | YES -- simplest initialization |
| `extension-module` | Build a `.so`/`.pyd` loaded by Python | NO -- we embed Python in Rust, not the other way around |
| `abi3-pyXX` | Stable ABI for Python extensions | NO -- only for extension modules |
| `multiple-pymethods` | Allow multiple `#[pymethods]` impl blocks | NO -- we don't export Rust types to Python |
| `gil-refs` | Enable deprecated GIL-bound reference API | NO -- use Bound API |

### `PYO3_PYTHON` Environment Variable

Controls which Python PyO3 links against at build time:

```bash
# Point to specific Python version
PYO3_PYTHON=python3.12 cargo build

# Or set in .cargo/config.toml for the project
```

```toml
# .cargo/config.toml
[env]
PYO3_PYTHON = "python3.12"
```

**Critical for chatter:** The system has Python 3.14, but qwen-tts requires 3.9-3.13. This config MUST be set.

### Build-Time Python Detection

PyO3 uses `pyo3-build-config` (pulled in automatically) to detect Python at build time. It checks:

1. `PYO3_PYTHON` env var (highest priority)
2. `python3` on PATH
3. `python` on PATH

It extracts:
- Python version
- Library path (for linking)
- Include path (for headers)

On macOS with Homebrew, Python 3.12 lives at `/opt/homebrew/bin/python3.12` or `/opt/homebrew/opt/python@3.12/bin/python3.12`.

### Cross-Platform Build Notes

| Platform | Python Headers | Notes |
|----------|---------------|-------|
| macOS (Homebrew) | Included with `python@3.12` formula | Set `PYO3_PYTHON=python3.12` |
| Ubuntu/Debian | `apt install python3.12-dev` | Need `-dev` package for headers |
| Arch Linux | `pacman -S python` | Headers included by default |

## 6. Thread Safety and GIL Management

### The GIL in Embedding Context

When you embed Python in Rust:

1. **`Python::attach()` acquires the GIL** (or attaches to the Python runtime in free-threading mode)
2. **All Python operations require the GIL** -- you can only call Python methods inside `attach()`
3. **The GIL is automatically released** when the `attach()` closure returns
4. **You can release the GIL early** with `py.allow_threads()`

### Pattern: Spinner + Python Operation

The main challenge for chatter: showing an animated spinner while Python does model loading.

```rust
use indicatif::{ProgressBar, ProgressStyle};
use std::time::Duration;

fn load_model_with_spinner(engine: &mut InferenceEngine) -> anyhow::Result<()> {
    // Create spinner BEFORE entering Python
    let spinner = ProgressBar::new_spinner();
    spinner.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.cyan} {msg} ({elapsed})")
            .expect("valid template")
    );
    spinner.set_message("Loading Qwen3-TTS 1.7B...");
    spinner.enable_steady_tick(Duration::from_millis(100));

    // indicatif spawns its own thread for animation.
    // That thread does NOT need the GIL -- it only writes to stderr.
    // So the spinner will animate while Python::attach holds the GIL.

    // This blocks the current thread but the spinner thread keeps ticking
    engine.load_model("Qwen/Qwen3-TTS-12Hz-1.7B-CustomVoice", "mps")?;

    spinner.finish_with_message("Model loaded");
    Ok(())
}
```

**Why this works:** `indicatif`'s `enable_steady_tick()` spawns a background thread that periodically redraws the spinner. This thread only does terminal I/O -- it never touches Python or the GIL. So even though `Python::attach()` holds the GIL on the main thread, the spinner thread runs independently.

### Pattern: Releasing the GIL for Rust Work

```rust
Python::attach(|py| {
    // Acquire model reference
    let model = self.model.as_ref().unwrap().bind(py);

    // Run inference (holds GIL -- Python code is running)
    let result = model.call_method1("generate", (text,))?;

    // Extract audio data to Rust (still needs GIL)
    let audio_bytes: Vec<u8> = result.call_method0("tobytes")?.extract()?;
    let sample_rate: u32 = model.getattr("config")?
        .getattr("sampling_rate")?.extract()?;

    // Release GIL for MP3 encoding (pure Rust, no Python needed)
    let mp3_data = py.allow_threads(|| {
        encode_to_mp3(&audio_bytes, sample_rate)
    })?;

    Ok(mp3_data)
})?;
```

`py.allow_threads()` temporarily releases the GIL, allowing other Python threads to run. For chatter, this matters when:
- Encoding audio to MP3 (pure Rust, no Python needed)
- Writing files to disk
- Any CPU-intensive Rust work after extracting data from Python

### What NOT to Do: Deadlock Patterns

```rust
// DEADLOCK: Do not nest attach() calls
Python::attach(|py| {
    Python::attach(|py2| {  // DEADLOCK -- already attached on this thread
        // ...
    });
    Ok(())
});

// DEADLOCK: Do not hold a Rust mutex while attaching on another thread
let data = Arc::new(Mutex::new(vec![]));
let data_clone = data.clone();

std::thread::spawn(move || {
    let guard = data_clone.lock().unwrap();  // Hold Rust mutex
    Python::attach(|py| {                    // Try to get GIL
        // ... DEADLOCK if main thread holds GIL and waits for mutex
    });
});

// SAFE: Use GILOnceCell instead of Mutex for Python objects
// SAFE: Use channels instead of shared mutexes between Python and Rust threads
```

**Note on `Python::attach()` re-entrancy:** On the same thread, `Python::attach()` should be re-entrant (it was for `with_gil`). But nesting is bad practice -- it makes GIL lifetime harder to reason about. Keep `attach()` calls at the top level of your engine functions.

## 7. Complete Embedding Example for Chatter

```rust
use pyo3::prelude::*;
use pyo3::types::PyDict;
use anyhow::{Context, Result};

/// Represents the Python bridge for chatter.
/// All PyO3 interaction is contained within this struct.
pub struct PythonBridge {
    model: Option<Py<PyAny>>,
}

impl PythonBridge {
    /// Create a new bridge. Initializes the Python interpreter.
    pub fn init() -> Result<Self> {
        // Set up Python path before initialization if needed
        // (e.g., inject virtualenv site-packages)
        if let Ok(venv) = std::env::var("VIRTUAL_ENV") {
            // Inject venv site-packages into PYTHONPATH
            let site_packages = format!("{venv}/lib/python3.12/site-packages");
            match std::env::var("PYTHONPATH") {
                Ok(existing) => std::env::set_var("PYTHONPATH",
                    format!("{site_packages}:{existing}")),
                Err(_) => std::env::set_var("PYTHONPATH", &site_packages),
            }
        }

        // Initialize Python explicitly (before auto-initialize kicks in)
        pyo3::prepare_freethreaded_python();

        Ok(Self { model: None })
    }

    /// Check if the Python environment has all required packages.
    pub fn check_environment(&self) -> Result<EnvironmentInfo> {
        Python::attach(|py| {
            let sys = py.import("sys")?;
            let python_version: String = sys.getattr("version")?.extract()?;

            let qwen_tts_version = py.import("qwen_tts")
                .and_then(|m| m.getattr("__version__"))
                .and_then(|v| v.extract::<String>())
                .ok();

            let torch = py.import("torch")
                .context("PyTorch not installed")?;
            let torch_version: String = torch.getattr("__version__")?.extract()?;

            let has_cuda: bool = torch.getattr("cuda")?
                .call_method0("is_available")?.extract()?;
            let has_mps: bool = torch.getattr("backends")?
                .getattr("mps")?
                .call_method0("is_available")?.extract()?;

            let gpu_name = if has_cuda {
                Some(torch.getattr("cuda")?
                    .call_method1("get_device_name", (0_i32,))?
                    .extract::<String>()?)
            } else {
                None
            };

            Ok(EnvironmentInfo {
                python_version,
                torch_version,
                qwen_tts_version,
                has_cuda,
                has_mps,
                gpu_name,
            })
        })
        .context("Failed to check Python environment")
    }

    /// Load a Qwen3-TTS model. Call this lazily -- only when inference is needed.
    pub fn load_model(&mut self, model_id: &str, backend: &ComputeBackend) -> Result<()> {
        Python::attach(|py| {
            let qwen_tts = py.import("qwen_tts")
                .context("qwen-tts package not found. Install with: pip install qwen-tts")?;
            let torch = py.import("torch")?;
            let model_class = qwen_tts.getattr("Qwen3TTSModel")?;

            let kwargs = PyDict::new(py);

            match backend {
                ComputeBackend::Cuda(ref _name) => {
                    kwargs.set_item("device_map", "cuda:0")?;
                    kwargs.set_item("dtype", torch.getattr("bfloat16")?)?;
                    // Only set flash_attention_2 on CUDA
                    kwargs.set_item("attn_implementation", "flash_attention_2")?;
                },
                ComputeBackend::Mps => {
                    kwargs.set_item("device_map", "mps")?;
                    // float16 for most models; float32 for Base (clone) models
                    let is_base_model = model_id.contains("Base");
                    let dtype = if is_base_model {
                        torch.getattr("float32")?
                    } else {
                        torch.getattr("float16")?
                    };
                    kwargs.set_item("dtype", dtype)?;
                },
                ComputeBackend::Cpu => {
                    kwargs.set_item("device_map", "cpu")?;
                    kwargs.set_item("dtype", torch.getattr("float32")?)?;
                },
            }

            let model = model_class.call_method("from_pretrained",
                (model_id,), Some(&kwargs))?;

            self.model = Some(model.unbind());
            Ok(())
        })
        .context(format!("Failed to load model: {model_id}"))
    }

    /// Check if a model is already downloaded in the HuggingFace cache.
    pub fn is_model_cached(&self, model_id: &str) -> Result<bool> {
        Python::attach(|py| {
            let code = format!(
                r#"
import os
from huggingface_hub import try_to_load_from_cache
result = try_to_load_from_cache("{model_id}", "config.json")
result is not None and not isinstance(result, str) or os.path.exists(str(result or ""))
"#
            );
            // Alternative simpler check:
            let huggingface_hub = py.import("huggingface_hub")?;
            let try_load = huggingface_hub.getattr("try_to_load_from_cache")?;
            let result = try_load.call1((model_id, "config.json"))?;

            // try_to_load_from_cache returns None if not cached, or the path if cached
            let is_cached = !result.is_none();
            Ok(is_cached)
        })
        .context("Failed to check model cache")
    }

    /// Download a model (lets HuggingFace handle its own progress output).
    pub fn download_model(&self, model_id: &str) -> Result<()> {
        Python::attach(|py| {
            let huggingface_hub = py.import("huggingface_hub")?;
            let snapshot_download = huggingface_hub.getattr("snapshot_download")?;

            // snapshot_download shows its own progress bars on stderr
            snapshot_download.call1((model_id,))?;
            Ok(())
        })
        .context(format!("Failed to download model: {model_id}"))
    }
}

pub struct EnvironmentInfo {
    pub python_version: String,
    pub torch_version: String,
    pub qwen_tts_version: Option<String>,
    pub has_cuda: bool,
    pub has_mps: bool,
    pub gpu_name: Option<String>,
}

pub enum ComputeBackend {
    Cuda(String),
    Mps,
    Cpu,
}
```

## 8. Embedding-Specific Gotchas

### Gotcha 1: Signal Handling Conflicts

**Problem:** Python installs its own signal handlers on initialization. This can interfere with Ctrl+C handling in the Rust CLI.

**Symptoms:** Ctrl+C doesn't cleanly exit, or produces a Python traceback instead of a clean Rust exit.

**Mitigation:**
```rust
// After Python initialization, restore Rust's default signal handler
pyo3::prepare_freethreaded_python();

// Reset SIGINT handler to default so Ctrl+C works as expected
// (Python's SIGINT handler raises KeyboardInterrupt, which PyO3 catches)
unsafe {
    libc::signal(libc::SIGINT, libc::SIG_DFL);
}
```

**Confidence: MEDIUM** -- This is a known issue from PyO3 discussions. The exact behavior may differ in 0.28.x. Test during implementation: if Ctrl+C works cleanly without the signal reset, don't add it.

### Gotcha 2: `sys.path` Does Not Include virtualenv

**Problem:** `prepare_freethreaded_python()` or `auto-initialize` starts Python with the system Python's `sys.path`. Packages in a virtualenv are invisible.

**Mitigation (from existing PITFALLS.md research):**
```rust
fn inject_virtualenv_if_active(py: Python<'_>) -> PyResult<()> {
    if let Ok(venv) = std::env::var("VIRTUAL_ENV") {
        let sys = py.import("sys")?;
        let path = sys.getattr("path")?;

        // Construct the site-packages path for the venv
        let site_packages = if cfg!(target_os = "windows") {
            format!("{}\\Lib\\site-packages", venv)
        } else {
            // Need to detect Python version for correct path
            let version_info = sys.getattr("version_info")?;
            let major: u32 = version_info.getattr("major")?.extract()?;
            let minor: u32 = version_info.getattr("minor")?.extract()?;
            format!("{}/lib/python{}.{}/site-packages", venv, major, minor)
        };

        // Insert at position 0 so venv packages take priority
        path.call_method1("insert", (0_i32, &site_packages))?;
    }
    Ok(())
}
```

Call this immediately after the first `Python::attach` or inside `PythonBridge::init()`.

### Gotcha 3: `atexit` and Python Finalization

**Problem:** Python's `atexit` handlers run when the interpreter finalizes. If PyTorch registers GPU cleanup via `atexit`, and the Rust binary exits before Python finalizes, GPU resources may leak or the process may hang on exit.

**Mitigation:** PyO3 handles Python finalization automatically when the process exits. However, for clean GPU cleanup:

```rust
impl Drop for PythonBridge {
    fn drop(&mut self) {
        // Drop the model reference before Python finalizes
        if let Some(model) = self.model.take() {
            Python::attach(|py| {
                // Explicitly drop the Python object
                drop(model);

                // Force GPU memory cleanup
                if let Ok(torch) = py.import("torch") {
                    if let Ok(cuda) = torch.getattr("cuda") {
                        let _ = cuda.call_method0("empty_cache");
                    }
                }

                // Run Python garbage collection
                if let Ok(gc) = py.import("gc") {
                    let _ = gc.call_method0("collect");
                }
            });
        }
    }
}
```

### Gotcha 4: `extension-module` Feature Causes Linker Errors

**Problem:** If `extension-module` is added to PyO3 features (by mistake or by following extension module tutorials), the binary won't link against `libpython` and will crash at runtime.

**Why:** `extension-module` tells PyO3 NOT to link against libpython (because Python extensions are loaded by Python, which already has libpython). For embedding, you NEED to link against libpython.

**Prevention:** Only use `auto-initialize`. Never use `extension-module` for an embedding binary.

### Gotcha 5: macOS Framework Python vs Homebrew Python

**Problem:** macOS ships a system Python framework. Homebrew installs its own Python. PyO3 might link against the wrong one.

**Symptoms:** Builds succeed but runtime `import` fails, or the wrong Python version runs.

**Prevention:**
```toml
# .cargo/config.toml -- force PyO3 to use Homebrew Python 3.12
[env]
PYO3_PYTHON = "/opt/homebrew/bin/python3.12"
```

Verify after build:
```rust
Python::attach(|py| {
    let sys = py.import("sys")?;
    let executable: String = sys.getattr("executable")?.extract()?;
    println!("Python executable: {executable}");
    // Should show /opt/homebrew/...python3.12, NOT /usr/bin/python3
    Ok(())
});
```

### Gotcha 6: Python Warnings Pollute stderr

**Problem:** Python packages (especially PyTorch and transformers) emit many warnings to stderr. These interleave with chatter's own output and indicatif spinners.

**Mitigation:**
```rust
fn suppress_python_warnings(py: Python<'_>) -> PyResult<()> {
    let warnings = py.import("warnings")?;
    // Suppress FutureWarning, DeprecationWarning from transformers
    warnings.call_method1("filterwarnings", ("ignore",))?;

    // Or more targeted:
    // warnings.call_method("filterwarnings", ("ignore",),
    //     Some(&[("category", py.get_type::<pyo3::exceptions::PyFutureWarning>())].into_py_dict(py)))?;

    // Suppress HuggingFace logging
    let os = py.import("os")?;
    let environ = os.getattr("environ")?;
    environ.set_item("TOKENIZERS_PARALLELISM", "false")?;
    environ.set_item("TRANSFORMERS_VERBOSITY", "error")?;

    Ok(())
}
```

Call this right after Python initialization, before any model imports.

## 9. Testing the PyO3 Bridge

### Unit Testing Without Python

The architecture pattern of isolating PyO3 to the `engine/` module means all other modules can be tested without Python:

```rust
// In engine/mod.rs -- the public API returns Rust types
pub struct AudioOutput {
    pub samples: Vec<f32>,
    pub sample_rate: u32,
}

// In audio/encode.rs -- can test with fixture data
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mp3_encoding() {
        let samples = vec![0.0f32; 24000]; // 1 second of silence at 24kHz
        let mp3 = encode_to_mp3(&samples, 24000).unwrap();
        assert!(!mp3.is_empty());
    }
}
```

### Integration Testing With Python

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use pyo3::prelude::*;

    #[test]
    fn test_python_available() {
        pyo3::prepare_freethreaded_python();
        Python::attach(|py| {
            let sys = py.import("sys").unwrap();
            let version: String = sys.getattr("version").unwrap().extract().unwrap();
            assert!(version.starts_with("3.12") || version.starts_with("3.13"));
        });
    }

    #[test]
    fn test_qwen_tts_importable() {
        pyo3::prepare_freethreaded_python();
        Python::attach(|py| {
            let result = py.import("qwen_tts");
            assert!(result.is_ok(), "qwen_tts should be importable");
        });
    }
}
```

**Note:** Tests that use `pyo3::prepare_freethreaded_python()` or `auto-initialize` must be careful about test parallelism. Python can only be initialized once per process. Use `cargo test -- --test-threads=1` or put all Python tests in a single test binary.

## 10. API Quick Reference

| Operation | PyO3 0.28.x Code |
|-----------|-------------------|
| Acquire GIL | `Python::attach(\|py\| { ... })` |
| Import module | `py.import("module_name")?` |
| Get attribute | `obj.getattr("name")?` |
| Set attribute | `obj.setattr("name", value)?` |
| Call method (no args) | `obj.call_method0("method")?` |
| Call method (positional) | `obj.call_method1("method", (arg1, arg2))?` |
| Call method (kwargs) | `obj.call_method("method", (arg1,), Some(&kwargs))?` |
| Create dict | `PyDict::new(py)` |
| Set dict item | `dict.set_item("key", value)?` |
| Extract to Rust | `py_obj.extract::<RustType>()?` |
| Store across GIL | `obj.unbind()` -> `Py<PyAny>` |
| Use stored object | `py_handle.bind(py)` -> `Bound<'py, PyAny>` |
| Release GIL | `py.allow_threads(\|\| { rust_work() })` |
| Run Python code | `py.run("code", None, None)?` |
| Eval Python expr | `py.eval("expr", None, None)?` |
| Check exception type | `err.is_instance_of::<PyImportError>(py)` |
| Get traceback | `err.traceback(py).and_then(\|tb\| tb.format().ok())` |
| Cache module | `GILOnceCell::get_or_try_init(py, \|\| ...)` |
| None value | `py.None()` |

## Confidence Assessment

| Area | Level | Reason |
|------|-------|--------|
| `auto-initialize` feature | HIGH | Confirmed in existing research, consistent with PyO3 docs |
| `Python::attach()` API | MEDIUM | Existing research confirms rename from `with_gil`. Exact signature should be verified at compile time. |
| `Py<PyAny>` / `.unbind()` / `.bind()` | HIGH | Core PyO3 pattern, well-documented, used in ARCHITECTURE.md examples |
| Error handling (`PyErr`) | HIGH | Standard PyO3 error type, patterns consistent across versions |
| `extension-module` caveat | HIGH | Well-documented: extension-module is for Python extensions, not embedding |
| Signal handling | MEDIUM | Known issue, but exact behavior in 0.28.x not verified via live docs |
| virtualenv `sys.path` injection | HIGH | Confirmed in PITFALLS.md with PyO3 GitHub issue references |
| GIL + spinner threading | HIGH | indicatif uses independent thread; confirmed compatible with GIL holding |
| `.tobytes()` for numpy arrays | MEDIUM | Standard numpy API; PyO3 extraction should work but buffer protocol may be more idiomatic |
| `GILOnceCell` | HIGH | Official PyO3 primitive, explicitly recommended over lazy_static |

## Sources

### From Existing Project Research (HIGH confidence)
- `.planning/research/STACK.md` -- PyO3 integration strategy, Cargo.toml setup
- `.planning/research/PITFALLS.md` -- Virtualenv detection, GIL deadlocks, PyO3 issues #3045, #3089, #3726, #4841
- `.planning/research/ARCHITECTURE.md` -- `InferenceEngine` pattern, `Py<PyAny>` caching, error boundary pattern
- `.planning/phases/01-foundation-and-python-bridge/01-RESEARCH.md` -- `Python::attach` rename, compute backend patterns

### Official Documentation (HIGH confidence, not re-verified due to tool limitations)
- [PyO3 User Guide](https://pyo3.rs/) -- Embedding chapter, GIL management, error handling
- [PyO3 0.28.2 API docs](https://docs.rs/pyo3/0.28.2/) -- `Python::attach`, `Py<T>`, `Bound<'py, T>`
- [PyO3 GitHub](https://github.com/PyO3/pyo3) -- Issues and discussions referenced in PITFALLS.md

### Training Data (MEDIUM confidence -- may be stale)
- PyO3 0.20-0.28 migration patterns
- `prepare_freethreaded_python()` semantics
- Signal handling behavior in embedded Python
- numpy buffer protocol via PyO3

---
*PyO3 embedding deep-dive for: chatter (Rust CLI with embedded Python for TTS)*
*Researched: 2026-03-27*
*Valid until: 2026-04-27 (30 days -- PyO3 0.28.x is stable)*
# PyO3 0.28.x Embedding Patterns -- Deep Dive

**Researched:** 2026-03-27
**Domain:** Embedding CPython in a Rust CLI binary via PyO3 0.28.2
**Confidence:** MEDIUM-HIGH (based on training data + existing project research; live web verification was unavailable)

## Summary

This document provides concrete, implementation-ready patterns for embedding Python in the `chatter` Rust CLI using PyO3 0.28.2. It covers initialization, calling the `qwen_tts` package, error handling, GIL management, build configuration, and known embedding gotchas.

PyO3 0.28 introduced a significant API rename: `Python::with_gil()` became `Python::attach()` to reflect that in Python 3.13+ free-threading mode, the GIL may not exist. The `auto-initialize` feature flag replaces explicit `prepare_freethreaded_python()` calls for embedding use cases.

**Key finding:** Do NOT use the `extension-module` feature for embedding. That feature is for Python extension modules (`.so`/`.pyd` files loaded by Python). For a Rust binary that embeds Python, use only `auto-initialize`.

---

## 1. Initialization: `auto-initialize` vs Manual

### Recommended for Chatter: Manual `prepare_freethreaded_python()`

Although `auto-initialize` is simpler, chatter needs to detect the correct Python site-packages path and set `PYTHONPATH` BEFORE the interpreter starts. This requires manual initialization.

```rust
use pyo3::prelude::*;

fn main() -> anyhow::Result<()> {
    // Set up Python environment BEFORE initialization
    configure_python_env();

    // Explicitly initialize Python interpreter
    pyo3::prepare_freethreaded_python();

    // Now safe to use Python
    Python::attach(|py| {
        let sys = py.import("sys")?;
        let version: String = sys.getattr("version")?.extract()?;
        println!("Python {version}");
        Ok(())
    })?;

    Ok(())
}

fn configure_python_env() {
    // Inject virtualenv site-packages if active
    if let Ok(venv) = std::env::var("VIRTUAL_ENV") {
        let site_packages = format!("{venv}/lib/python3.12/site-packages");
        match std::env::var("PYTHONPATH") {
            Ok(existing) => std::env::set_var(
                "PYTHONPATH",
                format!("{site_packages}:{existing}"),
            ),
            Err(_) => std::env::set_var("PYTHONPATH", &site_packages),
        }
    }

    // Suppress noisy Python warnings from transformers/pytorch
    std::env::set_var("TOKENIZERS_PARALLELISM", "false");
    std::env::set_var("TRANSFORMERS_VERBOSITY", "error");
}
```

### Alternative: `auto-initialize` (simpler but less control)

```toml
# Cargo.toml
[dependencies]
pyo3 = { version = "0.28", features = ["auto-initialize"] }
```

With `auto-initialize`, the Python interpreter starts automatically on the first call to `Python::attach()`. No explicit initialization code needed. However, you lose the ability to set environment variables before Python starts.

```rust
// auto-initialize: Python starts on first attach() call
Python::attach(|py| {
    // Python is already running here
    Ok(())
})?;
```

**Decision for chatter:** Use `auto-initialize` in Cargo.toml (so `prepare_freethreaded_python` is available) BUT call `prepare_freethreaded_python()` explicitly in `main()` after setting up env vars. The `auto-initialize` feature makes `prepare_freethreaded_python` available; calling it manually before the first `attach()` gives us control over timing.

### `Python::attach()` vs `Python::with_gil()` (API History)

| PyO3 Version | API | Notes |
|-------------|-----|-------|
| 0.20 - 0.27 | `Python::with_gil(\|py\| { ... })` | Original closure-based GIL acquisition |
| 0.28+ | `Python::attach(\|py\| { ... })` | Renamed. Same semantics. `with_gil` is a deprecated alias. |

The rename reflects Python 3.13's free-threading mode (PEP 703). `attach()` means "attach to the Python runtime" rather than "acquire the GIL," which is more accurate when the GIL might not exist.

**Confidence: MEDIUM** -- The rename is documented in existing project research (STACK.md mentions `Python::attach()`). If `attach()` produces a compile error in 0.28.2, fall back to `with_gil()`. Both should work; `attach()` is the forward-looking name.

---

## 2. Calling Python Packages from Rust

### Importing a Module

```rust
Python::attach(|py| {
    // Import a top-level module
    let qwen_tts = py.import("qwen_tts")?;

    // Import a submodule
    let torch = py.import("torch")?;

    // Access a class from a module (equivalent to: from qwen_tts import Qwen3TTSModel)
    let model_class = py.import("qwen_tts")?.getattr("Qwen3TTSModel")?;

    Ok(())
})?;
```

### Calling `Qwen3TTSModel.from_pretrained()`

This is a classmethod that takes positional args and keyword args:

```rust
use pyo3::prelude::*;
use pyo3::types::PyDict;

Python::attach(|py| {
    let qwen_tts = py.import("qwen_tts")?;
    let model_class = qwen_tts.getattr("Qwen3TTSModel")?;
    let torch = py.import("torch")?;

    // Build keyword arguments
    let kwargs = PyDict::new(py);
    kwargs.set_item("device_map", "mps")?;                    // or "cuda:0"
    kwargs.set_item("dtype", torch.getattr("float16")?)?;     // or bfloat16 for CUDA

    // Call: Qwen3TTSModel.from_pretrained("Qwen/Qwen3-TTS-12Hz-1.7B-CustomVoice",
    //                                     device_map=..., dtype=...)
    let model = model_class.call_method(
        "from_pretrained",
        ("Qwen/Qwen3-TTS-12Hz-1.7B-CustomVoice",),  // positional args as tuple
        Some(&kwargs),                                  // keyword args
    )?;

    // Store model for later use (see section 3)
    let model_handle: Py<PyAny> = model.unbind();

    Ok(())
})?;
```

### Calling Generation Methods

```rust
// generate_custom_voice(text, speaker, language, instruct)
let kwargs = PyDict::new(py);
kwargs.set_item("speaker", "Chelsie")?;
kwargs.set_item("language", "English")?;
kwargs.set_item("instruct", "Speak in a warm, friendly tone")?;

let results = model.call_method(
    "generate_custom_voice",
    ("Hello, this is a test.",),
    Some(&kwargs),
)?;

// generate_voice_design(text, language, instruct)
let kwargs = PyDict::new(py);
kwargs.set_item("language", "English")?;
kwargs.set_item("instruct", "A warm male voice with a slight British accent")?;

let results = model.call_method(
    "generate_voice_design",
    ("Hello, this is a test.",),
    Some(&kwargs),
)?;

// create_voice_clone_prompt(ref_audio, ref_text)
let kwargs = PyDict::new(py);
kwargs.set_item("ref_text", "The quick brown fox jumps over the lazy dog.")?;

let clone_prompt = model.call_method(
    "create_voice_clone_prompt",
    ("/path/to/reference.mp3",),
    Some(&kwargs),
)?;

// generate_voice_clone(text, language, voice_clone_prompt)
let kwargs = PyDict::new(py);
kwargs.set_item("language", "English")?;
kwargs.set_item("voice_clone_prompt", clone_prompt)?;

let results = model.call_method(
    "generate_voice_clone",
    ("Hello world",),
    Some(&kwargs),
)?;
```

### Method Call Variants (Quick Reference)

```rust
// No arguments
let result = obj.call_method0("method_name")?;

// Positional arguments only
let result = obj.call_method1("method_name", (arg1, arg2))?;

// Positional + keyword arguments
let result = obj.call_method("method_name", (arg1,), Some(&kwargs))?;
```

### Extracting Results Back to Rust

```rust
// Extract a Python string to Rust String
let version: String = sys.getattr("version")?.extract()?;

// Extract a Python int to Rust i64
let count: i64 = result.getattr("length")?.extract()?;

// Extract a Python bool
let has_cuda: bool = torch.getattr("cuda")?
    .call_method0("is_available")?
    .extract()?;

// Extract a Python list to Vec
let items: Vec<String> = py_list.extract()?;
```

### Extracting Audio Data (Critical for Chatter)

The generate methods return iterables of result objects with `.audio` (numpy array) and `.sr` (sample rate).

```rust
// Iterate over results (generator/iterable)
let results_list = results.call_method0("__iter__")?;

// Or convert to list first
let py_builtins = py.import("builtins")?;
let results_list = py_builtins.call_method1("list", (results,))?;
let first_result = results_list.get_item(0)?;

// Extract audio array and sample rate
let audio_np = first_result.getattr("audio")?;
let sample_rate: u32 = first_result.getattr("sr")?.extract()?;

// Option A: .tolist() -- simple but slow for large arrays
let samples: Vec<f32> = audio_np.call_method0("tolist")?.extract()?;

// Option B: .tobytes() + reinterpret -- fast for large arrays
let bytes: Vec<u8> = audio_np
    .call_method0("tobytes")?
    .extract()?;
let samples: Vec<f32> = bytes
    .chunks_exact(4)
    .map(|chunk| f32::from_le_bytes(chunk.try_into().unwrap()))
    .collect();
```

**Performance note:** Option B (tobytes + reinterpret) is significantly faster for large arrays. A 30-second audio clip at 24kHz has 720,000 float values. `.tolist()` creates 720K Python float objects; `.tobytes()` copies raw memory in one call.

**Confidence: MEDIUM** on the exact return type. The qwen-tts-api.md research notes: "Exact generate method return type -- generator vs list? `.audio`/`.sr` attributes vs tuple?" This needs hands-on validation. The code above handles both patterns.

---

## 3. Storing Python Objects Across GIL Boundaries

### `Py<PyAny>` -- The GIL-Independent Handle

When you need to keep a Python object alive outside of a `Python::attach` closure (e.g., caching the loaded model), use `Py<PyAny>`:

```rust
use pyo3::prelude::*;

pub struct InferenceEngine {
    model: Option<Py<PyAny>>,    // Survives across attach() calls
}

impl InferenceEngine {
    pub fn new() -> Self {
        Self { model: None }
    }

    pub fn load_model(&mut self, model_id: &str, device: &str) -> PyResult<()> {
        Python::attach(|py| {
            let qwen_tts = py.import("qwen_tts")?;
            let model_class = qwen_tts.getattr("Qwen3TTSModel")?;

            let kwargs = PyDict::new(py);
            kwargs.set_item("device_map", device)?;

            let model = model_class.call_method(
                "from_pretrained",
                (model_id,),
                Some(&kwargs),
            )?;

            // .unbind() converts Bound<'py, PyAny> -> Py<PyAny>
            // Releases the borrow on `py` but keeps the Python object alive
            self.model = Some(model.unbind());
            Ok(())
        })
    }

    pub fn generate(&self, text: &str) -> PyResult<Vec<f32>> {
        let model_ref = self.model.as_ref()
            .ok_or_else(|| pyo3::exceptions::PyRuntimeError::new_err(
                "Model not loaded"
            ))?;

        Python::attach(|py| {
            // .bind(py) converts Py<PyAny> -> Bound<'py, PyAny>
            let model = model_ref.bind(py);
            let result = model.call_method1("generate", (text,))?;
            let samples: Vec<f32> = result.call_method0("tolist")?.extract()?;
            Ok(samples)
        })
    }
}
```

**Key rules:**
- `Py<PyAny>` is `Send` (can cross thread boundaries)
- You MUST call `.bind(py)` inside `Python::attach` to use the object
- The `py` token proves you hold the GIL
- Use `.unbind()` to go from `Bound<'py, PyAny>` to `Py<PyAny>`

### `GILOnceCell` -- For One-Time Module Caching

For objects you initialize once and reuse (like imported modules):

```rust
use pyo3::sync::GILOnceCell;
use pyo3::prelude::*;

static TORCH_MODULE: GILOnceCell<Py<PyModule>> = GILOnceCell::new();

fn get_torch(py: Python<'_>) -> PyResult<&Bound<'_, PyModule>> {
    TORCH_MODULE
        .get_or_try_init(py, || {
            py.import("torch").map(|m| m.unbind())
        })
        .map(|m| m.bind(py))
}
```

**Why GILOnceCell over lazy_static/OnceLock:** `GILOnceCell` is designed for Python objects. Using `lazy_static` or `OnceLock` with `Python::attach` inside the init closure risks GIL deadlocks. `GILOnceCell` takes a `Python<'_>` token proving the GIL is already held.

**Confidence: HIGH** -- `GILOnceCell` is a well-documented PyO3 primitive. The PITFALLS.md explicitly recommends it over `lazy_static` or `OnceLock`.

---

## 4. Error Handling

### PyErr Structure

Every PyO3 operation that calls into Python returns `PyResult<T>` (`Result<T, PyErr>`). `PyErr` wraps a Python exception with its type, value, and traceback.

### Pattern A: `thiserror` at the Engine Boundary

```rust
use thiserror::Error;
use pyo3::prelude::*;

#[derive(Error, Debug)]
pub enum BridgeError {
    #[error("Python runtime error: {message}")]
    Python {
        message: String,
        traceback: Option<String>,
    },

    #[error("Module not found: {module} -- is qwen-tts installed?\n  Run: pip install qwen-tts")]
    ModuleNotFound { module: String },

    #[error("No GPU available -- Apple Silicon (MPS) or CUDA GPU required")]
    NoGpu,

    #[error("Model not loaded -- call load_model() first")]
    ModelNotLoaded,

    #[error("Audio encoding failed: {0}")]
    AudioEncode(String),

    #[error("{0}")]
    Io(#[from] std::io::Error),
}

impl From<PyErr> for BridgeError {
    fn from(err: PyErr) -> Self {
        Python::attach(|py| {
            // Check for specific Python exception types
            if err.is_instance_of::<pyo3::exceptions::PyImportError>(py) {
                return BridgeError::ModuleNotFound {
                    module: err.value(py).to_string(),
                };
            }
            if err.is_instance_of::<pyo3::exceptions::PyModuleNotFoundError>(py) {
                return BridgeError::ModuleNotFound {
                    module: err.value(py).to_string(),
                };
            }

            // Generic Python error with traceback
            let traceback = err.traceback(py)
                .and_then(|tb| tb.format().ok());

            BridgeError::Python {
                message: err.value(py).to_string(),
                traceback,
            }
        })
    }
}
```

### Pattern B: `anyhow` Context at the Call Site

```rust
use anyhow::{Context, Result};

fn check_python_env() -> Result<String> {
    Python::attach(|py| {
        let sys = py.import("sys")
            .context("Failed to import sys -- is Python correctly installed?")?;
        let version: String = sys.getattr("version")?.extract()?;
        Ok(version)
    })
    .context("Python runtime initialization failed")
}
```

### Combining Both (Recommended for Chatter)

- `thiserror` in `engine/` module for structured `BridgeError` variants
- `anyhow` in `main.rs` and CLI layer for adding human-readable context
- `From<PyErr> for BridgeError` keeps engine code clean (use `?` freely)
- CLI layer formats `BridgeError` differently based on `--verbose` flag

```rust
// In CLI layer:
match engine.load_model(model_id, device) {
    Ok(()) => { /* success */ },
    Err(BridgeError::ModuleNotFound { module }) => {
        eprintln!("Error: {module} not found.\n  Run: pip install qwen-tts");
        std::process::exit(1);
    },
    Err(BridgeError::Python { message, traceback }) => {
        eprintln!("Error: {message}");
        if verbose {
            if let Some(tb) = traceback {
                eprintln!("\nPython traceback:\n{tb}");
            }
        }
        std::process::exit(1);
    },
    Err(e) => {
        eprintln!("Error: {e}");
        std::process::exit(1);
    },
}
```

### Extracting Full Python Traceback

```rust
fn format_python_error(py: Python<'_>, err: &PyErr) -> String {
    let mut msg = err.value(py).to_string();
    if let Some(tb) = err.traceback(py) {
        if let Ok(formatted) = tb.format() {
            msg = format!("{formatted}\n{msg}");
        }
    }
    msg
}
```

Critical for the `--verbose` flag (decision D-05). Without tracebacks, debugging Python-side issues is nearly impossible.

---

## 5. Build Configuration

### Cargo.toml

```toml
[package]
name = "chatter"
version = "0.1.0"
edition = "2024"
rust-version = "1.85"

[dependencies]
pyo3 = { version = "0.28", features = ["auto-initialize"] }

# Do NOT add "extension-module" -- that's for Python extensions, not embedding
# Do NOT add "abi3" -- that's for extension modules distributed as wheels
# Do NOT add "gil-refs" -- deprecated API, use Bound<'py, T> instead
```

### Feature Flags Explained

| Feature | Purpose | Use for Chatter? |
|---------|---------|-----------------|
| `auto-initialize` | Auto-start Python on first `attach()` and provides `prepare_freethreaded_python()` | YES |
| `extension-module` | Build a `.so`/`.pyd` loaded by Python | NO -- causes linker errors in embedding |
| `abi3-pyXX` | Stable ABI for Python extensions | NO -- only for extension modules |
| `multiple-pymethods` | Multiple `#[pymethods]` impl blocks | NO -- we don't export Rust to Python |
| `gil-refs` | Enable deprecated GIL-bound reference API | NO -- use Bound API |

### `PYO3_PYTHON` Environment Variable

Controls which Python PyO3 links against at build time:

```bash
# Option 1: env var per command
PYO3_PYTHON=python3.12 cargo build

# Option 2: project-level config (recommended)
```

```toml
# .cargo/config.toml
[env]
PYO3_PYTHON = "python3.12"
```

**Critical for chatter on macOS:** Multiple Python versions likely exist (system framework Python, Homebrew Python 3.12, possibly conda). This config MUST be set to avoid linking against the wrong Python.

### Build-Time Python Detection

PyO3 uses `pyo3-build-config` (transitive dependency, pulled in automatically) to detect Python at build time. Priority order:

1. `PYO3_PYTHON` env var (highest priority)
2. `python3` on PATH
3. `python` on PATH

On macOS with Homebrew, Python 3.12 typically lives at:
- `/opt/homebrew/bin/python3.12` (Apple Silicon Homebrew)
- `/opt/homebrew/opt/python@3.12/bin/python3.12`

### Build Requirements by Platform

| Platform | Python Headers | Command |
|----------|---------------|---------|
| macOS (Homebrew) | Included with `python@3.12` | `brew install python@3.12` |
| Ubuntu/Debian | Separate `-dev` package | `apt install python3.12-dev` |
| Arch Linux | Headers included | `pacman -S python` |

---

## 6. Thread Safety and GIL Management

### The GIL in Embedding Context

1. **`Python::attach()` acquires the GIL** (or attaches to runtime in free-threading mode)
2. **All Python operations require the GIL** -- only callable inside `attach()`
3. **GIL is automatically released** when the `attach()` closure returns
4. **Release GIL early** with `py.allow_threads()` for pure-Rust work

### Pattern: Spinner While Python Runs (D-07, D-08)

The key challenge: showing an animated spinner with elapsed time while Python does model loading.

```rust
use indicatif::{ProgressBar, ProgressStyle};
use std::time::Duration;

fn load_model_with_spinner(engine: &mut InferenceEngine) -> anyhow::Result<()> {
    // Create spinner BEFORE entering Python
    let spinner = ProgressBar::new_spinner();
    spinner.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.cyan} {msg} ({elapsed})")
            .expect("valid template")
    );
    spinner.set_message("Loading Qwen3-TTS 1.7B...");
    spinner.enable_steady_tick(Duration::from_millis(100));

    // This blocks the current thread but the spinner thread keeps ticking.
    // indicatif::enable_steady_tick() spawns a background thread that only
    // writes to stderr -- it never touches Python or the GIL.
    engine.load_model("Qwen/Qwen3-TTS-12Hz-1.7B-CustomVoice", "mps")?;

    spinner.finish_with_message("Model loaded");
    Ok(())
}
```

**Why this works:** `indicatif`'s steady tick thread only does terminal I/O. It never calls into Python or touches the GIL. The GIL is held by the main thread during `engine.load_model()`, but the spinner thread runs independently. No deadlock.

### Pattern: Release GIL for Rust Work

```rust
Python::attach(|py| {
    let model = self.model.as_ref().unwrap().bind(py);

    // Run inference -- holds GIL (Python code is running)
    let result = model.call_method1("generate", (text,))?;

    // Extract audio data to Rust -- still needs GIL
    let audio_bytes: Vec<u8> = result
        .getattr("audio")?
        .call_method0("tobytes")?
        .extract()?;
    let sample_rate: u32 = result.getattr("sr")?.extract()?;

    // Release GIL for MP3 encoding (pure Rust, no Python needed)
    let mp3_data = py.allow_threads(|| {
        encode_to_mp3(&audio_bytes, sample_rate)
    })?;

    Ok(mp3_data)
})?;
```

`py.allow_threads()` temporarily releases the GIL, allowing other Python threads to run. Use it for:
- MP3 encoding (pure Rust)
- File I/O (writing output files)
- Any CPU-intensive Rust work after extracting data from Python

### Deadlock Patterns to Avoid

```rust
// DEADLOCK: Nested attach() calls
Python::attach(|py| {
    Python::attach(|py2| {  // DEADLOCK -- already attached on this thread
        // ...
    });
    Ok(())
});

// DEADLOCK: Rust mutex + GIL on separate threads
// Thread A: lock mutex -> attach (try GIL)
// Thread B: attach (hold GIL) -> lock mutex (try mutex)
// Classic lock ordering deadlock.

// SAFE alternatives:
// 1. Use GILOnceCell instead of Mutex for Python objects
// 2. Use channels (mpsc) instead of shared mutexes between threads
// 3. Always acquire GIL before mutex, never the reverse
```

**Re-entrancy note:** `Python::attach()` is re-entrant on the same thread (just like `with_gil` was). But nesting is bad practice -- it makes GIL lifetime harder to reason about. Keep `attach()` calls at the top level of engine functions.

---

## 7. Complete PythonBridge Implementation

This is the reference implementation for chatter's engine module:

```rust
use pyo3::prelude::*;
use pyo3::types::PyDict;
use anyhow::{Context, Result};

pub struct AudioOutput {
    pub samples: Vec<f32>,
    pub sample_rate: u32,
}

pub enum ComputeBackend {
    Cuda(String),  // GPU name
    Mps,
    Cpu,
}

pub struct EnvironmentInfo {
    pub python_version: String,
    pub torch_version: String,
    pub qwen_tts_version: Option<String>,
    pub has_cuda: bool,
    pub has_mps: bool,
    pub gpu_name: Option<String>,
}

/// All PyO3 interaction is contained within this struct.
/// No other module should import `pyo3` directly.
pub struct PythonBridge {
    model: Option<Py<PyAny>>,
}

impl PythonBridge {
    /// Create a new bridge. Initializes the Python interpreter.
    pub fn init() -> Result<Self> {
        configure_python_env();
        pyo3::prepare_freethreaded_python();

        // Suppress Python warnings immediately after init
        Python::attach(|py| {
            suppress_python_warnings(py)
        }).context("Failed to configure Python runtime")?;

        Ok(Self { model: None })
    }

    /// Full environment check for `chatter doctor` (D-09, D-10).
    pub fn check_environment(&self) -> Result<EnvironmentInfo> {
        Python::attach(|py| {
            let sys = py.import("sys")?;
            let python_version: String = sys.getattr("version")?.extract()?;

            let qwen_tts_version = py.import("qwen_tts")
                .and_then(|m| m.getattr("__version__"))
                .and_then(|v| v.extract::<String>())
                .ok();

            let torch = py.import("torch")
                .context("PyTorch not installed")?;
            let torch_version: String = torch.getattr("__version__")?.extract()?;

            let has_cuda: bool = torch.getattr("cuda")?
                .call_method0("is_available")?
                .extract()?;
            let has_mps: bool = torch.getattr("backends")?
                .getattr("mps")?
                .call_method0("is_available")?
                .extract()?;

            let gpu_name = if has_cuda {
                torch.getattr("cuda")?
                    .call_method1("get_device_name", (0_i32,))?
                    .extract::<String>()
                    .ok()
            } else {
                None
            };

            Ok(EnvironmentInfo {
                python_version,
                torch_version,
                qwen_tts_version,
                has_cuda,
                has_mps,
                gpu_name,
            })
        })
        .context("Failed to check Python environment")
    }

    /// Detect the best available compute backend.
    pub fn detect_backend(&self) -> Result<ComputeBackend> {
        Python::attach(|py| {
            let torch = py.import("torch")?;

            let has_cuda: bool = torch.getattr("cuda")?
                .call_method0("is_available")?
                .extract()?;
            if has_cuda {
                let name: String = torch.getattr("cuda")?
                    .call_method1("get_device_name", (0_i32,))?
                    .extract()?;
                return Ok(ComputeBackend::Cuda(name));
            }

            let has_mps: bool = torch.getattr("backends")?
                .getattr("mps")?
                .call_method0("is_available")?
                .extract()?;
            if has_mps {
                return Ok(ComputeBackend::Mps);
            }

            Ok(ComputeBackend::Cpu)
        })
        .context("Failed to detect compute backend")
    }

    /// Load a Qwen3-TTS model. Call lazily -- only when inference is needed (D-01).
    pub fn load_model(
        &mut self,
        model_id: &str,
        backend: &ComputeBackend,
    ) -> Result<()> {
        Python::attach(|py| {
            let qwen_tts = py.import("qwen_tts")
                .context("qwen-tts not found. Install: pip install qwen-tts")?;
            let torch = py.import("torch")?;
            let model_class = qwen_tts.getattr("Qwen3TTSModel")?;

            let kwargs = PyDict::new(py);

            match backend {
                ComputeBackend::Cuda(_) => {
                    kwargs.set_item("device_map", "cuda:0")?;
                    kwargs.set_item("dtype", torch.getattr("bfloat16")?)?;
                    // flash_attention_2 is optional (requires flash-attn package)
                    // Try it; fall back silently if not available
                    kwargs.set_item("attn_implementation", "flash_attention_2")?;
                },
                ComputeBackend::Mps => {
                    kwargs.set_item("device_map", "mps")?;
                    // CRITICAL: Base (clone) models need float32 on MPS.
                    // float16 causes NaN errors on MPS for clone models.
                    let is_base_model = model_id.contains("Base");
                    let dtype = if is_base_model {
                        torch.getattr("float32")?
                    } else {
                        torch.getattr("float16")?
                    };
                    kwargs.set_item("dtype", dtype)?;
                    // Do NOT set attn_implementation on MPS
                },
                ComputeBackend::Cpu => {
                    kwargs.set_item("device_map", "cpu")?;
                    kwargs.set_item("dtype", torch.getattr("float32")?)?;
                },
            }

            let model = model_class.call_method(
                "from_pretrained",
                (model_id,),
                Some(&kwargs),
            )?;

            self.model = Some(model.unbind());
            Ok(())
        })
        .context(format!("Failed to load model: {model_id}"))
    }

    /// Check if a model exists in the HuggingFace cache.
    pub fn is_model_cached(&self, model_id: &str) -> Result<bool> {
        Python::attach(|py| {
            let hf_hub = py.import("huggingface_hub")?;
            let try_load = hf_hub.getattr("try_to_load_from_cache")?;
            let result = try_load.call1((model_id, "config.json"))?;
            let is_cached = !result.is_none();
            Ok(is_cached)
        })
        .context("Failed to check model cache")
    }

    /// Download a model to the HuggingFace cache.
    /// HuggingFace shows its own progress output on stderr.
    pub fn download_model(&self, model_id: &str) -> Result<()> {
        Python::attach(|py| {
            let hf_hub = py.import("huggingface_hub")?;
            let snapshot_download = hf_hub.getattr("snapshot_download")?;
            snapshot_download.call1((model_id,))?;
            Ok(())
        })
        .context(format!("Failed to download model: {model_id}"))
    }

    pub fn is_model_loaded(&self) -> bool {
        self.model.is_some()
    }
}

/// Configure Python environment before interpreter starts.
fn configure_python_env() {
    // Inject virtualenv site-packages if VIRTUAL_ENV is set
    if let Ok(venv) = std::env::var("VIRTUAL_ENV") {
        // Note: the actual python version in the path needs runtime detection.
        // We use python3.12 as default per project requirements.
        let site_packages = format!("{venv}/lib/python3.12/site-packages");
        match std::env::var("PYTHONPATH") {
            Ok(existing) => std::env::set_var(
                "PYTHONPATH",
                format!("{site_packages}:{existing}"),
            ),
            Err(_) => std::env::set_var("PYTHONPATH", &site_packages),
        }
    }

    // Suppress noisy output from Python ML libraries
    std::env::set_var("TOKENIZERS_PARALLELISM", "false");
    std::env::set_var("TRANSFORMERS_VERBOSITY", "error");
}

/// Suppress Python warnings after interpreter is running.
fn suppress_python_warnings(py: Python<'_>) -> PyResult<()> {
    let warnings = py.import("warnings")?;
    warnings.call_method1("filterwarnings", ("ignore",))?;
    Ok(())
}

impl Drop for PythonBridge {
    fn drop(&mut self) {
        // Explicitly drop model reference for clean GPU memory release
        if let Some(model) = self.model.take() {
            let _ = Python::attach(|py| -> PyResult<()> {
                drop(model);

                // Force GPU memory cleanup
                if let Ok(torch) = py.import("torch") {
                    if let Ok(cuda) = torch.getattr("cuda") {
                        let _ = cuda.call_method0("empty_cache");
                    }
                    if let Ok(mps) = torch.getattr("mps") {
                        let _ = mps.call_method0("empty_cache");
                    }
                }

                // Run Python garbage collection
                if let Ok(gc) = py.import("gc") {
                    let _ = gc.call_method0("collect");
                }

                Ok(())
            });
        }
    }
}
```

---

## 8. Embedding-Specific Gotchas

### Gotcha 1: Signal Handling Conflicts

**Problem:** Python installs its own SIGINT handler on initialization. This can cause Ctrl+C to raise a Python `KeyboardInterrupt` instead of the Rust default behavior (immediate termination).

**Symptoms:** Ctrl+C produces a Python traceback or doesn't exit cleanly.

**Mitigation:**
```rust
pyo3::prepare_freethreaded_python();

// Reset SIGINT to default so Ctrl+C works as expected
unsafe {
    libc::signal(libc::SIGINT, libc::SIG_DFL);
}
```

**Confidence: MEDIUM** -- Known issue from PyO3 discussions. Test during implementation: if Ctrl+C works cleanly without the signal reset, skip it.

### Gotcha 2: `sys.path` Missing virtualenv

**Problem:** `prepare_freethreaded_python()` starts Python with system `sys.path`. Packages in a virtualenv are invisible.

**Mitigation:** Two layers of defense:

1. Set `PYTHONPATH` env var BEFORE `prepare_freethreaded_python()` (see `configure_python_env()` above)
2. Runtime `sys.path` injection as a fallback:

```rust
fn inject_virtualenv_if_needed(py: Python<'_>) -> PyResult<()> {
    if let Ok(venv) = std::env::var("VIRTUAL_ENV") {
        let sys = py.import("sys")?;
        let path = sys.getattr("path")?;

        // Build correct site-packages path
        let version_info = sys.getattr("version_info")?;
        let major: u32 = version_info.getattr("major")?.extract()?;
        let minor: u32 = version_info.getattr("minor")?.extract()?;
        let site_packages = if cfg!(target_os = "windows") {
            format!("{}\\Lib\\site-packages", venv)
        } else {
            format!("{}/lib/python{}.{}/site-packages", venv, major, minor)
        };

        // Insert at position 0 so venv packages take priority
        path.call_method1("insert", (0_i32, &site_packages))?;
    }
    Ok(())
}
```

**Confidence: HIGH** -- Confirmed in PITFALLS.md with PyO3 GitHub issue references (#3726, #4841).

### Gotcha 3: `atexit` and GPU Cleanup

**Problem:** If the Rust binary exits abruptly (panic, Ctrl+C) before Python finalization runs, PyTorch's GPU cleanup via `atexit` handlers may not execute. GPU resources can leak.

**Mitigation:** The `Drop` implementation on `PythonBridge` (shown in section 7) explicitly releases GPU memory. Additionally, PyO3 handles Python finalization automatically on normal process exit.

### Gotcha 4: `extension-module` Feature Causes Linker Errors

**Problem:** Adding `extension-module` to PyO3 features tells it NOT to link against `libpython`. For a Rust binary embedding Python, this causes unresolved symbol errors at link time or runtime crashes.

**Prevention:** Only use `auto-initialize` in Cargo.toml. Never `extension-module` for embedding. This is the single most common misconfiguration when starting with PyO3.

### Gotcha 5: macOS Framework Python vs Homebrew Python

**Problem:** macOS ships a system Python framework (`/usr/bin/python3`). Homebrew installs its own. PyO3 might link against the wrong one, causing "module not found" errors at runtime even though packages are installed.

**Prevention:**
```toml
# .cargo/config.toml
[env]
PYO3_PYTHON = "/opt/homebrew/bin/python3.12"
```

**Runtime verification:**
```rust
Python::attach(|py| {
    let sys = py.import("sys")?;
    let executable: String = sys.getattr("executable")?.extract()?;
    println!("Using Python at: {executable}");
    // Should show /opt/homebrew/... not /usr/bin/python3
    Ok(())
});
```

### Gotcha 6: Python Warnings Pollute stderr

**Problem:** PyTorch and transformers emit many warnings to stderr. These interleave with indicatif spinners and chatter's own output.

**Prevention:** Call `suppress_python_warnings()` immediately after Python initialization (shown in section 7). Also set env vars before Python starts:

```rust
std::env::set_var("TOKENIZERS_PARALLELISM", "false");
std::env::set_var("TRANSFORMERS_VERBOSITY", "error");
```

### Gotcha 7: `from_pretrained` Downloads on First Call

**Problem:** If the model is not cached, `from_pretrained()` silently downloads 3-7 GB of model weights. This looks like a hang to the user.

**Prevention:** Always check `is_model_cached()` before `load_model()`. If not cached, use `download_model()` with explicit messaging. This is why decision D-03 includes `chatter model download` as a separate command.

---

## 9. Testing the PyO3 Bridge

### Unit Tests Without Python

The architecture isolates PyO3 to `engine/` so everything else is testable without Python:

```rust
// audio/encode.rs
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mp3_encoding_silence() {
        let samples = vec![0.0f32; 24000]; // 1 second of silence
        let mp3 = encode_to_mp3(&samples, 24000).unwrap();
        assert!(!mp3.is_empty());
    }
}
```

### Integration Tests With Python

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use pyo3::prelude::*;

    #[test]
    fn test_python_available() {
        pyo3::prepare_freethreaded_python();
        Python::attach(|py| {
            let sys = py.import("sys").unwrap();
            let version: String = sys.getattr("version").unwrap().extract().unwrap();
            assert!(version.starts_with("3.12") || version.starts_with("3.13"));
        });
    }

    #[test]
    fn test_qwen_tts_importable() {
        pyo3::prepare_freethreaded_python();
        Python::attach(|py| {
            let result = py.import("qwen_tts");
            assert!(result.is_ok(), "qwen_tts should be importable");
        });
    }
}
```

**Critical note on test parallelism:** Python can only be initialized once per process. If multiple tests call `prepare_freethreaded_python()`, the second call is a no-op (safe but important to know). Use `cargo test -- --test-threads=1` for Python integration tests, or put all Python tests in a single test binary to avoid issues.

---

## 10. API Quick Reference

| Operation | PyO3 0.28.x Code |
|-----------|-------------------|
| Acquire GIL | `Python::attach(\|py\| { ... })` |
| Manual init | `pyo3::prepare_freethreaded_python()` |
| Import module | `py.import("module_name")?` |
| Get attribute | `obj.getattr("name")?` |
| Set attribute | `obj.setattr("name", value)?` |
| Call (no args) | `obj.call_method0("method")?` |
| Call (positional) | `obj.call_method1("method", (arg1, arg2))?` |
| Call (kwargs) | `obj.call_method("method", (arg1,), Some(&kwargs))?` |
| Create dict | `PyDict::new(py)` |
| Set dict item | `dict.set_item("key", value)?` |
| Extract to Rust | `py_obj.extract::<RustType>()?` |
| Store across GIL | `obj.unbind()` returns `Py<PyAny>` |
| Use stored object | `handle.bind(py)` returns `Bound<'py, PyAny>` |
| Release GIL | `py.allow_threads(\|\| { ... })` |
| Run Python code | `py.run("code", None, None)?` |
| Eval expression | `py.eval("expr", None, None)?` |
| Check exception type | `err.is_instance_of::<PyImportError>(py)` |
| Get traceback | `err.traceback(py).and_then(\|tb\| tb.format().ok())` |
| Cache module once | `GILOnceCell::get_or_try_init(py, \|\| ...)` |
| None value | `py.None()` |
| Create exception | `pyo3::exceptions::PyRuntimeError::new_err("msg")` |

---

## Confidence Assessment

| Area | Level | Reason |
|------|-------|--------|
| `auto-initialize` feature | HIGH | Confirmed in STACK.md, consistent across all project research |
| `Python::attach()` API | MEDIUM | Existing research confirms rename from `with_gil`. Exact compile behavior in 0.28.2 unverified -- fall back to `with_gil()` if needed |
| `Py<PyAny>` / `.unbind()` / `.bind()` | HIGH | Core PyO3 pattern documented in ARCHITECTURE.md examples |
| Error handling (`PyErr`, traceback) | HIGH | Standard PyO3 error type, patterns consistent across versions |
| `extension-module` must NOT be used | HIGH | Well-documented: extension-module is for Python extensions, not embedding |
| Signal handling conflicts | MEDIUM | Known issue from PyO3 discussions; exact 0.28.x behavior unverified |
| virtualenv `sys.path` injection | HIGH | Confirmed in PITFALLS.md with PyO3 GitHub issue #3726, #4841 |
| GIL + indicatif spinner threading | HIGH | indicatif uses independent thread for terminal I/O; no GIL interaction |
| `.tobytes()` for numpy arrays | MEDIUM | Standard numpy API; should work but buffer protocol may be more idiomatic |
| `GILOnceCell` for module caching | HIGH | Official PyO3 primitive, recommended in PITFALLS.md |
| qwen-tts return type (`.audio`, `.sr`) | MEDIUM | From qwen-tts-api.md research; flagged as needing hands-on validation |
| MPS dtype constraints | HIGH | From qwen-tts-api.md: Base models MUST use float32 on MPS |

## Open Questions (Need Hands-On Validation)

1. **`Python::attach()` vs `Python::with_gil()`** -- Does 0.28.2 compile with `attach()`? Is `with_gil` still available as deprecated alias? First compile attempt will answer this.
2. **qwen-tts generate return type** -- Generator or list? Attributes `.audio`/`.sr` or tuple unpacking? Must test with actual package.
3. **Signal handler behavior** -- Does Ctrl+C work cleanly without manual SIGINT reset? Test in built binary.
4. **Flash Attention on MPS** -- Is `attn_implementation` silently ignored on MPS, or does it error? Need to test.
5. **`huggingface_hub.try_to_load_from_cache` API** -- Exact return type for "not cached" case (None vs sentinel). Test with actual package.

## Sources

### From Existing Project Research (HIGH confidence)
- `.planning/research/STACK.md` -- PyO3 integration strategy, Cargo.toml features
- `.planning/research/PITFALLS.md` -- Venv detection (#3726, #4841), GIL deadlocks (#3045, #3089), GPU memory
- `.planning/research/ARCHITECTURE.md` -- InferenceEngine pattern, Py<PyAny> caching, error boundary, progress callback
- `.planning/phases/01-foundation-and-python-bridge/research/qwen-tts-api.md` -- Model variants, generation methods, MPS constraints

### Official Documentation (HIGH confidence, referenced but not re-fetched)
- [PyO3 User Guide](https://pyo3.rs/) -- Embedding chapter, GIL management
- [PyO3 0.28.2 API docs](https://docs.rs/pyo3/0.28.2/) -- `Python::attach`, `Py<T>`, `Bound<'py, T>`
- [PyO3 GitHub](https://github.com/PyO3/pyo3) -- Issues and discussions referenced in PITFALLS.md

### Training Data (MEDIUM confidence)
- PyO3 0.20-0.28 migration patterns (rename with_gil to attach)
- `prepare_freethreaded_python()` semantics and timing
- Signal handling behavior in embedded Python
- numpy `.tobytes()` / `.tolist()` extraction patterns
- `GILOnceCell` usage patterns

---
*PyO3 embedding deep-dive for: chatter (Rust CLI with embedded Python for TTS)*
*Researched: 2026-03-27*
*Valid until: 2026-04-27 (30 days -- PyO3 0.28.x is stable release)*
