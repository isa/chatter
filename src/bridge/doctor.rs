use pyo3::prelude::*;

use super::runtime::{detect_backend_inner, ComputeBackend};

/// All diagnostic information gathered by the doctor command.
pub struct SystemInfo {
    pub python_version: Option<String>,
    pub torch_version: Option<String>,
    pub qwen_tts_version: Option<String>,
    pub backend: Option<ComputeBackend>,
    pub disk_free_gb: Option<f64>,
    pub hf_cache_path: Option<String>,
    pub hf_cache_size_gb: Option<f64>,
}

/// Gather system information for the doctor command.
///
/// Uses a single `Python::attach` call to acquire the GIL once,
/// then runs all checks within that context. Each field is `Option<T>`
/// so any individual check can fail without crashing the whole report.
pub fn get_system_info() -> SystemInfo {
    Python::attach(|py| {
        let python_version = get_python_version(py);
        let torch_version = get_package_version(py, "torch");
        let qwen_tts_version = get_package_version(py, "qwen-tts");
        let backend = detect_backend_inner(py).ok();
        let (disk_free_gb, hf_cache_path, hf_cache_size_gb) = get_disk_info(py);

        SystemInfo {
            python_version,
            torch_version,
            qwen_tts_version,
            backend,
            disk_free_gb,
            hf_cache_path,
            hf_cache_size_gb,
        }
    })
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
