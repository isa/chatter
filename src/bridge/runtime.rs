use pyo3::prelude::*;

use super::error::BridgeError;

/// Detected compute backend, ordered by preference: CUDA > MLX > MPS > CPU.
#[derive(Debug, Clone)]
pub enum ComputeBackend {
    Cuda {
        name: String,
        vram_bytes: u64,
    },
    Mlx {
        memory_bytes: u64,
    },
    Mps,
    Cpu,
}

impl std::fmt::Display for ComputeBackend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Cuda { name, vram_bytes } => {
                write!(f, "CUDA ({}, {:.1} GB)", name, *vram_bytes as f64 / (1024.0 * 1024.0 * 1024.0))
            }
            Self::Mlx { memory_bytes } => {
                write!(f, "MLX ({:.1} GB)", *memory_bytes as f64 / (1024.0 * 1024.0 * 1024.0))
            }
            Self::Mps => write!(f, "MPS (Apple Silicon)"),
            Self::Cpu => write!(f, "CPU (no GPU acceleration)"),
        }
    }
}

/// Detect the best available compute backend.
///
/// Priority order per D-11: CUDA > MLX > MPS > CPU.
/// MLX is preferred over MPS on Apple Silicon because mlx-audio uses less memory.
pub fn detect_backend() -> Result<ComputeBackend, BridgeError> {
    Python::attach(|py| {
        // 1. Check CUDA
        match try_detect_cuda(py) {
            Ok(Some(backend)) => return Ok(backend),
            Ok(None) => {} // CUDA not available, continue
            Err(_) => {}   // torch not installed, will catch below
        }

        // 2. Check MLX (optional -- only on Apple Silicon with mlx installed)
        match try_detect_mlx(py) {
            Ok(Some(backend)) => return Ok(backend),
            Ok(None) => {} // MLX not available
            Err(_) => {}   // mlx not installed, skip
        }

        // 3. Check MPS (Apple Silicon via PyTorch)
        match try_detect_mps(py) {
            Ok(Some(backend)) => return Ok(backend),
            Ok(None) => {} // MPS not available
            Err(_) => {}   // torch not installed
        }

        // 4. Verify torch is at least importable for CPU fallback
        match py.import("torch") {
            Ok(_) => Ok(ComputeBackend::Cpu),
            Err(e) => {
                if e.is_instance_of::<pyo3::exceptions::PyModuleNotFoundError>(py) {
                    Err(BridgeError::QwenTtsNotInstalled)
                } else {
                    Err(BridgeError::Python(e))
                }
            }
        }
    })
}

/// Try to detect a CUDA GPU via PyTorch.
fn try_detect_cuda(py: Python<'_>) -> Result<Option<ComputeBackend>, BridgeError> {
    let torch = py.import("torch")?;
    let cuda = torch.getattr("cuda")?;
    let available: bool = cuda.call_method0("is_available")?.extract()?;

    if !available {
        return Ok(None);
    }

    let name: String = cuda.call_method1("get_device_name", (0,))?.extract()?;
    let props = cuda.call_method1("get_device_properties", (0,))?;
    let vram_bytes: u64 = props.getattr("total_memory")?.extract()?;

    Ok(Some(ComputeBackend::Cuda { name, vram_bytes }))
}

/// Try to detect MLX Metal backend (Apple Silicon only).
fn try_detect_mlx(py: Python<'_>) -> Result<Option<ComputeBackend>, BridgeError> {
    let mx = match py.import("mlx.core") {
        Ok(m) => m,
        Err(_) => return Ok(None), // mlx not installed, skip
    };

    let metal = mx.getattr("metal")?;
    let available: bool = metal.call_method0("is_available")?.extract()?;

    if !available {
        return Ok(None);
    }

    let device_info = metal.call_method0("device_info")?;
    let memory_bytes: u64 = device_info.get_item("memory_size")?.extract()?;

    Ok(Some(ComputeBackend::Mlx { memory_bytes }))
}

/// Try to detect MPS backend (Apple Silicon via PyTorch).
fn try_detect_mps(py: Python<'_>) -> Result<Option<ComputeBackend>, BridgeError> {
    let torch = py.import("torch")?;
    let backends = torch.getattr("backends")?;
    let mps = backends.getattr("mps")?;
    let available: bool = mps.call_method0("is_available")?.extract()?;

    if available {
        Ok(Some(ComputeBackend::Mps))
    } else {
        Ok(None)
    }
}
