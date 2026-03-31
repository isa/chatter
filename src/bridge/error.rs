/// Error types for the Python bridge layer.
///
/// These errors cover Python runtime issues, missing dependencies,
/// compute backend problems, and model management failures.
#[derive(Debug, thiserror::Error)]
pub enum BridgeError {
    #[error("Python error: {0}")]
    Python(#[from] pyo3::PyErr),

    #[error("qwen-tts is not installed. Install it with: pip install qwen-tts")]
    QwenTtsNotInstalled,

    #[error("Python runtime not found. Ensure Python 3.13 is installed")]
    PythonNotFound,

    #[error("Python venv not found. Set CHATTER_VENV=/path/to/venv or reinstall: brew reinstall chatter")]
    VenvNotFound,

    #[error("CHATTER_VENV={0} does not contain a valid Python venv (missing bin/python)")]
    InvalidVenv(String),

    #[error("No GPU available. Chatter requires Apple Silicon (MLX/MPS) or a CUDA GPU")]
    NoGpuAvailable,

    #[error("Model not found: {0}")]
    ModelNotFound(String),

    #[error("{0}")]
    Other(String),

    #[error("Voice design failed: {0}")]
    VoiceDesignFailed(String),

    #[error("Voice clone failed: {0}")]
    VoiceCloneFailed(String),

    #[error("Speech generation failed: {0}")]
    GenerationFailed(String),

    #[error("Audio encoding failed: {0}")]
    AudioEncodingFailed(String),

    #[error("Profile error: {0}")]
    ProfileError(String),

    #[error("Inference backend not available: {0}")]
    BackendNotAvailable(String),
}
