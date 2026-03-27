/// Error types for the Python bridge layer.
#[derive(Debug, thiserror::Error)]
pub enum BridgeError {
    #[error("Python error: {0}")]
    Python(#[from] pyo3::PyErr),

    #[error("qwen-tts is not installed. Install it with: pip install qwen-tts")]
    QwenTtsNotInstalled,

    #[error("Python runtime not found. Ensure Python 3.12 is installed")]
    PythonNotFound,

    #[error("No GPU available. Chatter requires Apple Silicon (MLX/MPS) or a CUDA GPU")]
    NoGpuAvailable,

    #[error("Model not found: {0}")]
    ModelNotFound(String),

    #[error("{0}")]
    Other(String),
}
