use std::path::PathBuf;
use std::process::Command;

use directories::ProjectDirs;

use super::error::BridgeError;

/// The packages to install in the chatter venv.
const REQUIRED_PACKAGES: &[&str] = &["qwen-tts"];

/// Get the path to chatter's managed venv.
///
/// Location: `~/.local/share/chatter/venv/` (Linux)
///           `~/Library/Application Support/chatter/venv/` (macOS)
pub fn venv_path() -> Result<PathBuf, BridgeError> {
    let dirs = ProjectDirs::from("", "", "chatter")
        .ok_or_else(|| BridgeError::Other("Could not determine data directory".to_string()))?;
    Ok(dirs.data_dir().join("venv"))
}

/// Get the site-packages path inside the venv.
pub fn venv_site_packages() -> Result<PathBuf, BridgeError> {
    let venv = venv_path()?;
    // On macOS/Linux: venv/lib/pythonX.Y/site-packages
    // We glob for the python version directory
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
    Err(BridgeError::Other("Could not find site-packages in venv".to_string()))
}

/// Check if the managed venv exists and has qwen-tts installed.
pub fn is_venv_ready() -> bool {
    let Ok(venv) = venv_path() else {
        return false;
    };
    let python = venv_python_path(&venv);
    if !python.exists() {
        return false;
    }
    // Quick check: can we import qwen_tts?
    let output = Command::new(&python)
        .args(["-c", "import qwen_tts"])
        .output();
    matches!(output, Ok(o) if o.status.success())
}

/// Get the path to the Python binary inside the venv.
fn venv_python_path(venv: &std::path::Path) -> PathBuf {
    venv.join("bin").join("python")
}

/// Find the system Python 3 binary (from Homebrew or system).
fn find_system_python() -> Result<String, BridgeError> {
    // Try common paths in priority order
    let candidates = [
        "python3.12",
        "python3.13",
        "python3.11",
        "python3",
    ];

    for candidate in &candidates {
        let output = Command::new(candidate)
            .args(["--version"])
            .output();
        if let Ok(o) = output {
            if o.status.success() {
                return Ok(candidate.to_string());
            }
        }
    }

    Err(BridgeError::PythonNotFound)
}

/// Create the managed venv and install required packages.
///
/// Returns the venv path on success. The caller should show progress UI.
pub fn create_venv() -> Result<PathBuf, BridgeError> {
    let venv = venv_path()?;
    let python = find_system_python()?;

    // Create parent directories
    if let Some(parent) = venv.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| BridgeError::Other(format!("Failed to create data directory: {e}")))?;
    }

    // Create venv
    let status = Command::new(&python)
        .args(["-m", "venv", &venv.to_string_lossy()])
        .status()
        .map_err(|e| BridgeError::Other(format!("Failed to create venv: {e}")))?;

    if !status.success() {
        return Err(BridgeError::Other("Python venv creation failed".to_string()));
    }

    // Install packages using the venv's pip
    let venv_pip = venv.join("bin").join("pip");
    let status = Command::new(&venv_pip)
        .args(["install", "--upgrade", "pip"])
        .status()
        .map_err(|e| BridgeError::Other(format!("Failed to upgrade pip: {e}")))?;

    if !status.success() {
        return Err(BridgeError::Other("pip upgrade failed".to_string()));
    }

    let mut pip_args = vec!["install"];
    for pkg in REQUIRED_PACKAGES {
        pip_args.push(pkg);
    }
    let status = Command::new(&venv_pip)
        .args(&pip_args)
        .status()
        .map_err(|e| BridgeError::Other(format!("Failed to install packages: {e}")))?;

    if !status.success() {
        return Err(BridgeError::Other(
            "Package installation failed. Check your internet connection and try again.".to_string(),
        ));
    }

    Ok(venv)
}

/// Configure the Python runtime to use the managed venv's packages.
///
/// Must be called BEFORE `Python::attach()` or any PyO3 operations.
/// Sets PYTHONPATH so that the embedded Python interpreter can find
/// packages installed in the chatter venv.
pub fn configure_python_for_venv() -> Result<(), BridgeError> {
    let site_packages = venv_site_packages()?;
    // Set PYTHONPATH so PyO3's embedded interpreter picks up venv packages.
    // SAFETY: This is called early in main(), before any threads are spawned
    // and before Python is initialized. No concurrent env access is possible.
    unsafe { std::env::set_var("PYTHONPATH", &site_packages) };
    Ok(())
}
