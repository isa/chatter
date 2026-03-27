pub mod error;
pub mod model;
pub mod runtime;

pub use error::BridgeError;
pub use model::{download_model, list_cached_models, ModelInfo};
pub use runtime::{detect_backend, ComputeBackend};
