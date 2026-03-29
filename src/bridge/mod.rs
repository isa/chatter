pub mod doctor;
pub mod error;
pub mod inference;
pub mod model;
pub mod runtime;
pub mod venv;

pub use doctor::{get_system_info, SystemInfo};
pub use error::BridgeError;
pub use inference::{
    create_and_save_clone_prompt, generate_speech, set_engine, unload_all_models,
    voice_clone_from_audio, voice_design,
};
pub use model::{
    download_model, list_cached_chatterbox_models, list_cached_models, ModelInfo, ModelQuantization,
};
pub use runtime::{detect_backend, ComputeBackend};
pub use venv::{
    configure_python_for_venv, diagnose_venv, ensure_bridge_installed, is_venv_ready, venv_path,
    VenvDiagnosis,
};
