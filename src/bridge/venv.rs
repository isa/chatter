use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::process::{Command, Stdio};

use directories::ProjectDirs;
use indicatif::ProgressBar;

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
    let output = Command::new(&python)
        .args(["-c", "import qwen_tts"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .output();
    matches!(output, Ok(o) if o.status.success())
}

/// Get the path to the Python binary inside the venv.
fn venv_python_path(venv: &std::path::Path) -> PathBuf {
    venv.join("bin").join("python")
}

/// Find the system Python 3 binary (from Homebrew or system).
fn find_system_python() -> Result<String, BridgeError> {
    let candidates = [
        "python3.14",
        "python3.13",
        "python3.12",
        "python3.11",
        "python3",
    ];

    for candidate in &candidates {
        let output = Command::new(candidate)
            .args(["--version"])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
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
/// Takes an optional spinner to update with live status from pip.
/// All subprocess output is captured — nothing leaks to the terminal.
pub fn create_venv(spinner: Option<&ProgressBar>) -> Result<PathBuf, BridgeError> {
    let venv = venv_path()?;
    let python = find_system_python()?;

    // Create parent directories
    if let Some(parent) = venv.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| BridgeError::Other(format!("Failed to create data directory: {e}")))?;
    }

    // Create venv (quiet — no useful output to show)
    update_spinner(spinner, "Creating Python virtual environment...");
    let output = Command::new(&python)
        .args(["-m", "venv", &venv.to_string_lossy()])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .map_err(|e| BridgeError::Other(format!("Failed to create venv: {e}")))?;

    if !output.status.success() {
        return Err(BridgeError::Other("Python venv creation failed".to_string()));
    }

    // Upgrade pip (quiet)
    let venv_pip = venv.join("bin").join("pip");
    update_spinner(spinner, "Upgrading pip...");
    run_pip_quiet(&venv_pip, &["install", "--upgrade", "pip"], spinner)?;

    // Install packages with live status
    update_spinner(spinner, "Installing qwen-tts (this may take a few minutes)...");
    let mut pip_args = vec!["install"];
    for pkg in REQUIRED_PACKAGES {
        pip_args.push(pkg);
    }
    run_pip_with_progress(&venv_pip, &pip_args, spinner)?;

    Ok(venv)
}

/// Run pip silently, only reporting failure.
fn run_pip_quiet(pip: &std::path::Path, args: &[&str], spinner: Option<&ProgressBar>) -> Result<(), BridgeError> {
    let output = Command::new(pip)
        .args(args)
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .output()
        .map_err(|e| BridgeError::Other(format!("Failed to run pip: {e}")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let _ = spinner; // spinner is managed by caller
        return Err(BridgeError::Other(format!("pip failed: {stderr}")));
    }
    Ok(())
}

/// Run pip and stream stderr to extract "Collecting/Downloading/Installing" lines
/// for the spinner status. All output is captured — nothing goes to the terminal.
fn run_pip_with_progress(pip: &std::path::Path, args: &[&str], spinner: Option<&ProgressBar>) -> Result<(), BridgeError> {
    let mut child = Command::new(pip)
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| BridgeError::Other(format!("Failed to run pip: {e}")))?;

    // Read stderr in a thread to avoid blocking
    let stderr = child.stderr.take();
    let stdout = child.stdout.take();

    let spinner_clone = spinner.cloned();
    let stderr_thread = std::thread::spawn(move || {
        let Some(stderr) = stderr else { return String::new() };
        let reader = BufReader::new(stderr);
        let mut full_output = String::new();
        for line in reader.lines() {
            let Ok(line) = line else { continue };
            full_output.push_str(&line);
            full_output.push('\n');

            // Extract useful status from pip output
            if let Some(status) = extract_pip_status(&line) {
                if let Some(ref sp) = spinner_clone {
                    sp.set_message(status);
                }
            }
        }
        full_output
    });

    // Drain stdout silently
    let stdout_thread = std::thread::spawn(move || {
        let Some(stdout) = stdout else { return };
        let reader = BufReader::new(stdout);
        for _ in reader.lines() {}
    });

    let status = child.wait()
        .map_err(|e| BridgeError::Other(format!("pip process error: {e}")))?;

    let stderr_output = stderr_thread.join().unwrap_or_default();
    let _ = stdout_thread.join();

    if !status.success() {
        // Show last few lines of stderr for debugging
        let last_lines: String = stderr_output
            .lines()
            .rev()
            .take(5)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect::<Vec<_>>()
            .join("\n");
        return Err(BridgeError::Other(format!(
            "Package installation failed:\n{last_lines}\n\nCheck your internet connection and try again."
        )));
    }
    Ok(())
}

/// Extract a short status message from a pip output line.
fn extract_pip_status(line: &str) -> Option<String> {
    let trimmed = line.trim();

    if trimmed.starts_with("Collecting ") {
        // "Collecting torch>=2.0.0 (from qwen-tts)" → "Installing torch..."
        let pkg = trimmed
            .strip_prefix("Collecting ")?
            .split([' ', '>', '<', '=', '!', ';', '('])
            .next()?;
        Some(format!("Installing {pkg}..."))
    } else if trimmed.starts_with("Downloading ") {
        // "Downloading torch-2.6.0-cp314-..." → "Downloading torch..."
        let file = trimmed
            .strip_prefix("Downloading ")?
            .split(['/', ' '])
            .last()?;
        // Extract package name from filename (before first -)
        let pkg = file.split('-').next().unwrap_or(file);
        // Check for size info in parentheses
        if let Some(size_start) = trimmed.rfind('(') {
            let size_info = &trimmed[size_start..];
            Some(format!("Downloading {pkg} {size_info}"))
        } else {
            Some(format!("Downloading {pkg}..."))
        }
    } else if trimmed.starts_with("Installing collected") {
        Some("Installing collected packages...".to_string())
    } else if trimmed.starts_with("Successfully installed") {
        let count = trimmed.split_whitespace().count() - 2; // "Successfully installed" + packages
        Some(format!("Successfully installed {count} packages"))
    } else {
        None
    }
}

fn update_spinner(spinner: Option<&ProgressBar>, msg: &str) {
    if let Some(sp) = spinner {
        sp.set_message(msg.to_string());
    }
}

/// Configure the Python runtime to use the managed venv's packages.
///
/// Must be called BEFORE `Python::attach()` or any PyO3 operations.
pub fn configure_python_for_venv() -> Result<(), BridgeError> {
    let site_packages = venv_site_packages()?;
    // SAFETY: Called early in main(), before any threads are spawned
    // and before Python is initialized. No concurrent env access.
    unsafe { std::env::set_var("PYTHONPATH", &site_packages) };
    Ok(())
}
