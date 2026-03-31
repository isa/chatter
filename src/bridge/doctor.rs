use pyo3::prelude::*;

use super::runtime::{detect_backend_inner, ComputeBackend};

/// All diagnostic information gathered by the doctor command.
pub struct SystemInfo {
    pub python_version: Option<String>,
    pub backend: Option<ComputeBackend>,
    /// The inference package version (mlx-audio on MLX, qwen-tts otherwise).
    pub inference_pkg_name: String,
    pub inference_pkg_version: Option<String>,
    pub disk_free_gb: Option<f64>,
    pub hf_cache_path: Option<String>,
    pub hf_cache_size_gb: Option<f64>,
    /// The installed version of the `chatterbox-tts` package, or None if not installed.
    pub chatterbox_pkg_version: Option<String>,
    /// Whether the ChatterBox package is importable.
    pub chatterbox_installed: bool,
    /// Whether key scientific stack imports succeed (e.g. scipy.special).
    ///
    /// This catches situations where package versions are present but importing
    /// the dependency graph fails at runtime (common with NumPy/SciPy wheel ABI mismatches).
    pub python_imports_ok: bool,
    /// If python_imports_ok is false, this contains a short error string.
    pub python_imports_error: Option<String>,
}

/// Gather system information for the doctor command.
///
/// Uses a single `Python::attach` call to acquire the GIL once,
/// then runs all checks within that context. Each field is `Option<T>`
/// so any individual check can fail without crashing the whole report.
pub fn get_system_info() -> SystemInfo {
    Python::attach(|py| {
        // Suppress deprecation warnings (e.g., mx.metal.device_info)
        let _ = py.run(
            pyo3::ffi::c_str!("import warnings; warnings.filterwarnings('ignore')"),
            None,
            None,
        );
        let python_version = get_python_version(py);
        let backend = detect_backend_inner(py).ok();

        // Check the correct inference package based on detected backend
        let is_mlx = matches!(backend, Some(ComputeBackend::Mlx { .. }));
        let (inference_pkg_name, pip_name) = if is_mlx {
            ("mlx-audio".to_string(), "mlx-audio")
        } else {
            ("qwen-tts".to_string(), "qwen-tts")
        };
        let inference_pkg_version = get_package_version(py, pip_name);

        let (disk_free_gb, hf_cache_path, hf_cache_size_gb) = get_disk_info(py);

        // Check ChatterBox installation via package metadata (lightweight, no heavy import)
        let chatterbox_pkg_version = get_package_version(py, "chatterbox-tts");
        let chatterbox_installed = chatterbox_pkg_version.is_some();

        let (python_imports_ok, python_imports_error) = check_python_imports(py);

        SystemInfo {
            python_version,
            backend,
            inference_pkg_name,
            inference_pkg_version,
            disk_free_gb,
            hf_cache_path,
            hf_cache_size_gb,
            chatterbox_pkg_version,
            chatterbox_installed,
            python_imports_ok,
            python_imports_error,
        }
    })
}

fn check_python_imports(py: Python<'_>) -> (bool, Option<String>) {
    let code = pyo3::ffi::c_str!(
        r#"
import importlib

for mod in ("numpy", "scipy.special"):
    importlib.import_module(mod)
"#
    );

    match py.run(code, None, None) {
        Ok(_) => (true, None),
        Err(e) => {
            // Keep it short for CLI output.
            let msg = e.to_string();
            let first_line = msg.lines().next().unwrap_or(&msg).to_string();
            (false, Some(first_line))
        }
    }
}

/// Get the Python version string (first line of sys.version).
fn get_python_version(py: Python<'_>) -> Option<String> {
    let sys = py.import("sys").ok()?;
    let version: String = sys.getattr("version").ok()?.extract().ok()?;
    Some(version.lines().next().unwrap_or(&version).to_string())
}

/// Get a package version using importlib.metadata (lightweight, avoids heavy imports).
fn get_package_version(py: Python<'_>, package: &str) -> Option<String> {
    let metadata = py.import("importlib.metadata").ok()?;
    let version = metadata.call_method1("version", (package,)).ok()?;
    version.extract().ok()
}

/// Get disk space info: free space, HF cache path, and cache size.
fn get_disk_info(py: Python<'_>) -> (Option<f64>, Option<String>, Option<f64>) {
    let disk_free_gb = get_disk_free_gb(py);
    let (hf_cache_path, hf_cache_size_gb) = get_hf_cache_info(py);
    (disk_free_gb, hf_cache_path, hf_cache_size_gb)
}

/// Get free disk space in GB using shutil.disk_usage.
fn get_disk_free_gb(py: Python<'_>) -> Option<f64> {
    let shutil = py.import("shutil").ok()?;
    let usage = shutil.call_method1("disk_usage", ("/",)).ok()?;
    let free: u64 = usage.getattr("free").ok()?.extract().ok()?;
    Some(free as f64 / 1_000_000_000.0)
}

/// Get HuggingFace cache path and approximate size.
fn get_hf_cache_info(py: Python<'_>) -> (Option<String>, Option<f64>) {
    let os_mod = match py.import("os") {
        Ok(m) => m,
        Err(_) => return (None, None),
    };

    // Determine HF cache path: HF_HOME env var or default
    let environ = match os_mod.getattr("environ") {
        Ok(e) => e,
        Err(_) => return (None, None),
    };

    let hf_home: String = match environ.call_method1("get", ("HF_HOME", "")) {
        Ok(val) => val.extract().unwrap_or_default(),
        Err(_) => String::new(),
    };

    let cache_path = if hf_home.is_empty() {
        let path_mod = match os_mod.getattr("path") {
            Ok(p) => p,
            Err(_) => return (None, None),
        };
        let home: String = match path_mod.call_method1("expanduser", ("~/.cache/huggingface",)) {
            Ok(p) => p.extract().unwrap_or_default(),
            Err(_) => return (None, None),
        };
        home
    } else {
        hf_home
    };

    if cache_path.is_empty() {
        return (None, None);
    }

    // Calculate approximate cache size by walking hub/ directory
    let cache_size_gb = calculate_cache_size(&cache_path);

    (Some(cache_path), cache_size_gb)
}

/// Walk the HF cache hub directory and sum file sizes. Cap at 5 seconds.
fn calculate_cache_size(cache_path: &str) -> Option<f64> {
    use std::fs;
    use std::path::Path;
    use std::time::Instant;

    let hub_path = Path::new(cache_path).join("hub");
    if !hub_path.is_dir() {
        return Some(0.0);
    }

    let start = Instant::now();
    let mut total: u64 = 0;

    fn walk_dir(dir: &Path, total: &mut u64, start: &Instant) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            if start.elapsed().as_secs_f64() > 5.0 {
                return;
            }
            let path = entry.path();
            if path.is_dir() {
                walk_dir(&path, total, start);
            } else if let Ok(meta) = fs::metadata(&path) {
                *total += meta.len();
            }
        }
    }

    walk_dir(&hub_path, &mut total, &start);
    Some(total as f64 / 1_000_000_000.0)
}
