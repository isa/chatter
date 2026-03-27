pub mod doctor;
pub mod error;
pub mod model;
pub mod runtime;

pub use doctor::{get_system_info, SystemInfo};
pub use error::BridgeError;
pub use model::{download_model, list_cached_models, ModelInfo};
pub use runtime::{detect_backend, ComputeBackend};
