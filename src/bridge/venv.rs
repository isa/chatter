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
    /// Venv directory exists but `bin/python` is missing.
    NoPython { venv_path: PathBuf },
    /// Python exists but `import chatter_bridge` fails.
    BridgeMissing { venv_path: PathBuf },
}

/// The chatter_bridge.py source, embedded at compile time.
const BRIDGE_MODULE_SOURCE: &str = include_str!("../../chatter_bridge.py");

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
        return Err(BridgeError::Other(format!(
            "CHATTER_VENV={path} does not contain a valid Python venv"
        )));
    }

    // 2. Binary-relative (Homebrew: bin/chatter → ../libexec/venv/)
    if let Ok(exe) = std::env::current_exe() {
        if let Some(bin_dir) = exe.parent() {
            let brew_venv = bin_dir.parent().map(|p| p.join("libexec").join("venv"));
            if let Some(ref venv) = brew_venv {
                if venv.join("bin").join("python").exists() {
                    return Ok(venv.clone());
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
    let Ok(venv) = venv_path() else {
        return VenvDiagnosis::NotFound;
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

/// Ensure the chatter_bridge.py module is installed and up-to-date in the venv.
///
/// Compares the embedded source against the installed copy. Writes (or overwrites)
/// if missing or stale. This handles upgrades across chatter versions.
pub fn ensure_bridge_installed() -> Result<(), BridgeError> {
    let site_packages = venv_site_packages()?;
    let dest = site_packages.join("chatter_bridge.py");
    let needs_write = if dest.exists() {
        std::fs::read_to_string(&dest).map_or(true, |existing| existing != BRIDGE_MODULE_SOURCE)
    } else {
        true
    };
    if needs_write {
        std::fs::write(&dest, BRIDGE_MODULE_SOURCE)
            .map_err(|e| BridgeError::Other(format!("Failed to install chatter_bridge.py: {e}")))?;
    }
    Ok(())
}

/// Get the path to the Python binary inside the venv.
fn venv_python_path(venv: &std::path::Path) -> PathBuf {
    venv.join("bin").join("python")
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
