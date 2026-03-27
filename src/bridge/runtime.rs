use pyo3::prelude::*;

use super::error::BridgeError;

/// Detected compute backend for model inference.
#[derive(Debug, Clone)]
pub enum ComputeBackend {
    Cuda { name: String, vram_bytes: u64 },
    Mlx { memory_bytes: u64 },
    Mps,
    Cpu,
}

/// Detect the best available compute backend.
///
/// Priority order per D-11: CUDA > MLX > MPS > CPU.
pub fn detect_backend() -> Result<ComputeBackend, BridgeError> {
    Python::attach(|py| detect_backend_inner(py))
}

/// Inner detection logic that runs within an existing GIL context.
/// This can be called from within another `Python::attach` closure
/// to avoid nested GIL acquisition.
pub(crate) fn detect_backend_inner(py: pyo3::Python<'_>) -> Result<ComputeBackend, BridgeError> {
    // Try CUDA first
    match py.import("torch") {
        Ok(torch) => {
            let cuda_mod = torch.getattr("cuda")?;
            let cuda_available: bool = cuda_mod.call_method0("is_available")?.extract()?;
            if cuda_available {
                let name: String = cuda_mod
                    .call_method1("get_device_name", (0,))?
                    .extract()?;
                let props = cuda_mod.call_method1("get_device_properties", (0,))?;
                let vram_bytes: u64 = props.getattr("total_memory")?.extract()?;
                return Ok(ComputeBackend::Cuda { name, vram_bytes });
            }

            // Try MLX
            if let Ok(mx) = py.import("mlx.core") {
                if let Ok(metal) = mx.getattr("metal") {
                    let available: bool = metal.call_method0("is_available")?.extract()?;
                    if available {
                        let info = metal.call_method0("device_info")?;
                        let memory_bytes: u64 = info
                            .get_item("memory_size")?
                            .extract()?;
                        return Ok(ComputeBackend::Mlx { memory_bytes });
                    }
                }
            }

            // Try MPS
            let backends = torch.getattr("backends")?;
            let mps = backends.getattr("mps")?;
            let mps_available: bool = mps.call_method0("is_available")?.extract()?;
            if mps_available {
                return Ok(ComputeBackend::Mps);
            }

            Ok(ComputeBackend::Cpu)
        }
        Err(e) => {
            if e.is_instance_of::<pyo3::exceptions::PyModuleNotFoundError>(py) {
                Err(BridgeError::QwenTtsNotInstalled)
            } else {
                Err(BridgeError::Python(e))
            }
        }
    }
}
