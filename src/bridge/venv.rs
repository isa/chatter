use std::path::PathBuf;
use std::process::{Command, Stdio};

use super::error::BridgeError;

/// Diagnostic result for venv health checks.
#[derive(Debug)]
pub enum VenvDiagnosis {
    /// Venv is fully functional.
    Ready,
    /// No venv path could be resolved.
    NotFound,
    /// `CHATTER_VENV` is set but points to an invalid path.
    InvalidEnvVar { value: String },
    /// Venv directory exists but `bin/python` is missing.
    NoPython { venv_path: PathBuf },
    /// Python exists but `import chatter_bridge` fails.
    BridgeMissing { venv_path: PathBuf },
}

/// Curated ChatterBox dependencies, embedded at compile time.
const CHATTERBOX_REQUIREMENTS: &str = include_str!("../../requirements/chatterbox.txt");

/// Pinned versions — keep in sync with `requirements-mlx.txt`.
const PIN_NUMPY: &str = "2.2.6";
const PIN_SCIPY: &str = "1.16.2";

/// The chatter_bridge package sources, embedded at compile time.
const BRIDGE_INIT: &str = include_str!("../../chatter_bridge/__init__.py");
const BRIDGE_ENGINES_INIT: &str = include_str!("../../chatter_bridge/engines/__init__.py");
const BRIDGE_ENGINES_QWEN: &str = include_str!("../../chatter_bridge/engines/qwen.py");
const BRIDGE_ENGINES_CHATTERBOX: &str = include_str!("../../chatter_bridge/engines/chatterbox.py");

/// Discover the venv path. Resolution order:
///
/// 1. `CHATTER_VENV` env var (explicit override for dev/testing)
/// 2. Binary-relative `../libexec/venv/` (Homebrew Cellar layout)
/// 3. Error — venv must be provided by the installation method
///
/// Chatter never creates a venv at runtime. `brew install chatter`
/// sets up the venv in the Cellar during formula installation.
pub fn venv_path() -> Result<PathBuf, BridgeError> {
    // 1. Explicit override
    if let Ok(path) = std::env::var("CHATTER_VENV") {
        let p = PathBuf::from(&path);
        if p.join("bin").join("python").exists() {
            return Ok(p);
        }
        return Err(BridgeError::InvalidVenv(path));
    }

    // 2. Binary-relative (Homebrew: bin/chatter → ../libexec/venv/)
    //    Try canonicalized path first (resolves Homebrew symlinks), then raw path.
    for exe in [
        std::env::current_exe().and_then(|p| p.canonicalize()),
        std::env::current_exe(),
    ]
    .into_iter()
    .flatten()
    {
        if let Some(bin_dir) = exe.parent() {
            if let Some(prefix) = bin_dir.parent() {
                let venv = prefix.join("libexec").join("venv");
                if venv.join("bin").join("python").exists() {
                    return Ok(venv);
                }
            }
        }
    }

    Err(BridgeError::VenvNotFound)
}

/// Get the site-packages path inside the venv.
pub fn venv_site_packages() -> Result<PathBuf, BridgeError> {
    let venv = venv_path()?;
    let lib_dir = venv.join("lib");
    if lib_dir.is_dir() {
        for entry in std::fs::read_dir(&lib_dir).map_err(|e| BridgeError::Other(e.to_string()))? {
            let entry = entry.map_err(|e| BridgeError::Other(e.to_string()))?;
            let name = entry.file_name();
            let name_str = name.to_string_lossy();
            if name_str.starts_with("python") {
                let sp = entry.path().join("site-packages");
                if sp.is_dir() {
                    return Ok(sp);
                }
            }
        }
    }
    // Provide a helpful error showing what's actually in lib/
    let lib_contents: Vec<String> = if lib_dir.is_dir() {
        std::fs::read_dir(&lib_dir)
            .ok()
            .map(|entries| {
                entries
                    .flatten()
                    .map(|e| e.file_name().to_string_lossy().to_string())
                    .collect()
            })
            .unwrap_or_default()
    } else {
        vec![]
    };
    Err(BridgeError::Other(format!(
        "Could not find site-packages in venv at {}. lib/ contains: [{}]",
        venv.display(),
        lib_contents.join(", ")
    )))
}

/// Check if the venv is found and has chatter_bridge importable.
pub fn is_venv_ready() -> bool {
    let Ok(venv) = venv_path() else {
        return false;
    };
    let python = venv_python_path(&venv);
    if !python.exists() {
        return false;
    }
    let output = Command::new(&python)
        .args(["-c", "import chatter_bridge"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .output();
    matches!(output, Ok(o) if o.status.success())
}

/// Diagnose the venv state for the doctor command.
///
/// Returns a structured diagnosis that allows the doctor command to show
/// specific, actionable error messages instead of a generic "not found".
pub fn diagnose_venv() -> VenvDiagnosis {
    let venv = match venv_path() {
        Ok(v) => v,
        Err(BridgeError::InvalidVenv(value)) => {
            return VenvDiagnosis::InvalidEnvVar { value };
        }
        Err(_) => return VenvDiagnosis::NotFound,
    };
    let python = venv_python_path(&venv);
    if !python.exists() {
        return VenvDiagnosis::NoPython {
            venv_path: venv,
        };
    }
    let output = Command::new(&python)
        .args(["-c", "import chatter_bridge"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .output();
    if matches!(output, Ok(ref o) if o.status.success()) {
        VenvDiagnosis::Ready
    } else {
        VenvDiagnosis::BridgeMissing {
            venv_path: venv,
        }
    }
}

/// Ensure the chatter_bridge package is installed and up-to-date in the venv.
///
/// Compares the embedded sources against the installed copies. Writes (or overwrites)
/// if missing or stale. This handles upgrades across chatter versions.
/// Also cleans up the old single-file chatter_bridge.py if present (v1.0 upgrade path).
pub fn ensure_bridge_installed() -> Result<(), BridgeError> {
    let site_packages = venv_site_packages()?;

    // Clean up old single-file bridge if it exists (v1.0 upgrade path).
    // A stale chatter_bridge.py would shadow the package directory.
    let old_single_file = site_packages.join("chatter_bridge.py");
    if old_single_file.is_file() {
        std::fs::remove_file(&old_single_file).map_err(|e| {
            BridgeError::Other(format!("Failed to remove old chatter_bridge.py: {e}"))
        })?;
    }

    // Create package directory structure
    let pkg_dir = site_packages.join("chatter_bridge");
    let engines_dir = pkg_dir.join("engines");
    std::fs::create_dir_all(&engines_dir).map_err(|e| {
        BridgeError::Other(format!("Failed to create chatter_bridge package dirs: {e}"))
    })?;

    // Write each file if missing or stale
    let files: [(PathBuf, &str); 4] = [
        (pkg_dir.join("__init__.py"), BRIDGE_INIT),
        (engines_dir.join("__init__.py"), BRIDGE_ENGINES_INIT),
        (engines_dir.join("qwen.py"), BRIDGE_ENGINES_QWEN),
        (engines_dir.join("chatterbox.py"), BRIDGE_ENGINES_CHATTERBOX),
    ];
    for (dest, source) in &files {
        let needs_write = if dest.exists() {
            std::fs::read_to_string(dest).map_or(true, |existing| existing != *source)
        } else {
            true
        };
        if needs_write {
            std::fs::write(dest, source).map_err(|e| {
                BridgeError::Other(format!(
                    "Failed to install {}: {e}",
                    dest.display()
                ))
            })?;
        }
    }
    Ok(())
}

/// Get the path to the Python binary inside the venv.
fn venv_python_path(venv: &std::path::Path) -> PathBuf {
    venv.join("bin").join("python")
}

/// Install ChatterBox TTS dependencies into the managed venv.
///
/// Uses `pip install --no-deps` for chatterbox-tts itself (to avoid pulling in
/// gradio and resemble-perth), then installs the curated dependency list.
/// On Apple Silicon, also installs `mlx-audio` for MLX backend support.
pub fn install_chatterbox_deps() -> Result<(), BridgeError> {
    let venv = venv_path()?;
    let pip = venv_python_path(&venv);

    // Step 1: Install chatterbox-tts with --no-deps
    let output = Command::new(&pip)
        .args(["-m", "pip", "install", "--no-deps", "chatterbox-tts==0.1.7"])
        .stdout(Stdio::inherit())
        .stderr(Stdio::piped())
        .output()
        .map_err(|e| BridgeError::Other(format!("Failed to run pip: {e}")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(BridgeError::Other(format!(
            "Failed to install chatterbox-tts: {stderr}"
        )));
    }

    // Step 2: Write curated requirements to a temp file and install
    let temp_dir = std::env::temp_dir();
    let req_file = temp_dir.join("chatter-cb-requirements.txt");
    std::fs::write(&req_file, CHATTERBOX_REQUIREMENTS).map_err(|e| {
        BridgeError::Other(format!("Failed to write temp requirements file: {e}"))
    })?;

    let output = Command::new(&pip)
        .args([
            "-m",
            "pip",
            "install",
            "--ignore-installed",
            "-r",
            &req_file.to_string_lossy(),
        ])
        .stdout(Stdio::inherit())
        .stderr(Stdio::piped())
        .output()
        .map_err(|e| BridgeError::Other(format!("Failed to run pip: {e}")))?;

    // Clean up temp file regardless of result
    let _ = std::fs::remove_file(&req_file);

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(BridgeError::Other(format!(
            "Failed to install ChatterBox dependencies: {stderr}"
        )));
    }

    // Step 3: On Apple Silicon macOS, install mlx-audio for MLX backend
    if cfg!(target_os = "macos") && cfg!(target_arch = "aarch64") {
        let output = Command::new(&pip)
            .args(["-m", "pip", "install", "mlx-audio>=0.2.8"])
            .stdout(Stdio::inherit())
            .stderr(Stdio::piped())
            .output()
            .map_err(|e| BridgeError::Other(format!("Failed to run pip: {e}")))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(BridgeError::Other(format!(
                "Failed to install mlx-audio: {stderr}"
            )));
        }
    }

    // Step 4: Re-assert NumPy/SciPy pins last — mlx-audio or transitive deps can
    // upgrade to incompatible pairs that break `import scipy.special`.
    reinstall_pinned_numpy_scipy()?;

    Ok(())
}

/// Force the known-good NumPy/SciPy pair (matches `requirements-mlx.txt`).
/// Call after ChatterBox install or from `doctor --fix` when imports fail.
pub fn reinstall_pinned_numpy_scipy() -> Result<(), BridgeError> {
    let venv = venv_path()?;
    let pip = venv_python_path(&venv);
    let numpy_spec = format!("numpy=={PIN_NUMPY}");
    let scipy_spec = format!("scipy=={PIN_SCIPY}");

    let output = Command::new(&pip)
        .args([
            "-m",
            "pip",
            "install",
            "--no-cache-dir",
            numpy_spec.as_str(),
            scipy_spec.as_str(),
        ])
        .stdout(Stdio::inherit())
        .stderr(Stdio::piped())
        .output()
        .map_err(|e| BridgeError::Other(format!("Failed to run pip: {e}")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(BridgeError::Other(format!(
            "Failed to pin numpy/scipy: {stderr}"
        )));
    }
    Ok(())
}

/// Configure the Python runtime to use the venv's packages.
///
/// Must be called BEFORE any PyO3 operations. Sets up:
/// - PYTHONPATH → venv site-packages
/// - sys.argv → neutralized (prevents libraries from parsing Rust CLI args)
/// - sys.executable → venv Python (prevents `-c` errors in subprocess spawning)
/// - Various env vars to suppress noisy Python library output
pub fn configure_python_for_venv() -> Result<(), BridgeError> {
    let site_packages = venv_site_packages()?;
    // SAFETY: Called early in main(), before any threads are spawned
    // and before Python is initialized. No concurrent env access.
    unsafe { std::env::set_var("PYTHONPATH", &site_packages) };
    // Suppress torchaudio's noisy SoX backend probe
    unsafe { std::env::set_var("TORCHAUDIO_BACKEND", "soundfile") };
    // Suppress tokenizers parallelism warning
    unsafe { std::env::set_var("TOKENIZERS_PARALLELISM", "false") };
    // Suppress Python warnings in child processes (multiprocessing resource_tracker)
    unsafe { std::env::set_var("PYTHONWARNINGS", "ignore") };

    let venv = venv_path()?;
    let python_path = venv_python_path(&venv);
    let python_path_str = python_path.to_string_lossy().to_string();

    pyo3::Python::attach(|py| {
        use pyo3::prelude::PyAnyMethods;
        let sys = py.import("sys").expect("sys module");
        let argv_list = pyo3::types::PyList::new(py, &["chatter"]).expect("list creation");
        sys.as_any().setattr("argv", argv_list).expect("set sys.argv");
        sys.as_any()
            .setattr("executable", python_path_str.as_str())
            .expect("set sys.executable");
    });

    Ok(())
}
