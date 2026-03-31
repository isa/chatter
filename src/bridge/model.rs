use pyo3::prelude::*;
use pyo3::types::PyIterator;

use super::error::BridgeError;
use super::runtime::ComputeBackend;
use crate::cli::ModelVariant;

/// Model quantization level for MLX models.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelQuantization {
    Bf16,
    EightBit,
}

impl ModelQuantization {
    /// Return the MLX repo suffix for this quantization level.
    pub fn mlx_suffix(&self) -> &'static str {
        match self {
            Self::Bf16 => "bf16",
            Self::EightBit => "8bit",
        }
    }
}

impl From<ModelVariant> for ModelQuantization {
    fn from(v: ModelVariant) -> Self {
        match v {
            ModelVariant::Bf16 => ModelQuantization::Bf16,
            ModelVariant::EightBit => ModelQuantization::EightBit,
        }
    }
}

/// Information about a cached model.
pub struct ModelInfo {
    pub repo_id: String,
    pub size_label: String,
    pub local_path: Option<String>,
    pub size_bytes: Option<u64>,
    /// Engine this model belongs to: "qwen" or "chatterbox".
    pub engine: String,
    /// Human-readable variant label for ChatterBox models (e.g. "Original", "Turbo").
    pub variant_label: Option<String>,
}

/// All Qwen3-TTS 1.7B model variants.
/// Returns MLX variants if backend is MLX, otherwise PyTorch variants.
pub fn model_variants(backend: &ComputeBackend, quant: &ModelQuantization) -> Vec<String> {
    match backend {
        ComputeBackend::Mlx { .. } => {
            let suffix = quant.mlx_suffix();
            vec![
                format!("mlx-community/Qwen3-TTS-12Hz-1.7B-VoiceDesign-{suffix}"),
                format!("mlx-community/Qwen3-TTS-12Hz-1.7B-CustomVoice-{suffix}"),
                format!("mlx-community/Qwen3-TTS-12Hz-1.7B-Base-{suffix}"),
            ]
        }
        _ => vec![
            "Qwen/Qwen3-TTS-12Hz-1.7B-VoiceDesign".to_string(),
            "Qwen/Qwen3-TTS-12Hz-1.7B-CustomVoice".to_string(),
            "Qwen/Qwen3-TTS-12Hz-1.7B-Base".to_string(),
        ],
    }
}

/// All ChatterBox model variants (HuggingFace repo IDs).
pub fn chatterbox_model_variants() -> Vec<String> {
    vec![
        "ResembleAI/chatterbox".to_string(),
        "mlx-community/chatterbox-fp16".to_string(),
        "mlx-community/chatterbox-turbo-fp16".to_string(),
    ]
}

/// Detect which quantization variant is cached for the given backend.
/// Prefers 8bit if both are cached. Returns Bf16 as fallback if nothing is cached
/// (so that inference auto-detect has a sensible default).
pub fn detect_cached_quantization(backend: &ComputeBackend) -> Result<ModelQuantization, BridgeError> {
    match backend {
        ComputeBackend::Mlx { .. } => {
            // Check if 8bit models are cached
            let eightbit_variants = model_variants(backend, &ModelQuantization::EightBit);
            let cached = list_cached_models()?;
            let has_8bit = eightbit_variants.iter().any(|v| cached.iter().any(|c| c.repo_id == *v));
            if has_8bit {
                return Ok(ModelQuantization::EightBit);
            }
            let bf16_variants = model_variants(backend, &ModelQuantization::Bf16);
            let has_bf16 = bf16_variants.iter().any(|v| cached.iter().any(|c| c.repo_id == *v));
            if has_bf16 {
                return Ok(ModelQuantization::Bf16);
            }
            // Nothing cached; default to 8bit
            Ok(ModelQuantization::EightBit)
        }
        _ => Ok(ModelQuantization::Bf16), // Non-MLX backends don't have quantization variants
    }
}

/// Return a human-readable label for the model size.
pub fn size_label() -> &'static str {
    "1.7B"
}

/// Import huggingface_hub, returning a friendly error if not installed.
fn import_hf_hub(py: Python<'_>) -> Result<Bound<'_, PyModule>, BridgeError> {
    match py.import("huggingface_hub") {
        Ok(m) => Ok(m),
        Err(e) => {
            if e.is_instance_of::<pyo3::exceptions::PyModuleNotFoundError>(py) {
                Err(BridgeError::QwenTtsNotInstalled)
            } else {
                Err(BridgeError::Python(e))
            }
        }
    }
}

/// Derive a human-readable variant label from a ChatterBox repo ID.
fn chatterbox_variant_label(repo_id: &str) -> Option<String> {
    if repo_id == "ResembleAI/chatterbox" {
        Some("Original".to_string())
    } else if repo_id.contains("chatterbox-turbo") {
        Some("Turbo".to_string())
    } else if repo_id.contains("chatterbox") && repo_id.contains("multilingual") {
        Some("Multilingual".to_string())
    } else if repo_id.contains("chatterbox") {
        Some("ChatterBox".to_string())
    } else {
        None
    }
}

/// Check available disk space and return estimated download size for ChatterBox models.
///
/// Returns `(free_bytes, estimated_download_bytes)`.
/// Estimated ChatterBox total size is approximately 20 GB for all 3 variants.
pub fn disk_space_check() -> Result<(u64, u64), BridgeError> {
    let estimated: u64 = 20_000_000_000;
    let free = Python::attach(|py| -> Result<u64, BridgeError> {
        let shutil = py.import("shutil").map_err(|e| {
            BridgeError::Other(format!("Failed to check disk space: {e}"))
        })?;
        let usage = shutil.call_method1("disk_usage", ("/",)).map_err(|e| {
            BridgeError::Other(format!("Failed to check disk space: {e}"))
        })?;
        let free_bytes: u64 = usage.getattr("free").map_err(|e| {
            BridgeError::Other(format!("Failed to check disk space: {e}"))
        })?.extract().map_err(|e| {
            BridgeError::Other(format!("Failed to check disk space: {e}"))
        })?;
        Ok(free_bytes)
    })?;
    Ok((free, estimated))
}

/// Download all 1.7B model variants from HuggingFace.
///
/// Detects the compute backend to choose PyTorch or MLX variants.
/// Uses `huggingface_hub.snapshot_download()` which downloads model files
/// to the default HF cache (`~/.cache/huggingface/hub/`).
pub fn download_model(quant: &ModelQuantization) -> Result<(), BridgeError> {
    let backend = super::runtime::detect_backend()?;
    let variants = model_variants(&backend, quant);

    Python::attach(|py| {
        let hf_hub = import_hf_hub(py)?;
        let snapshot_download = hf_hub.getattr("snapshot_download")?;

        for (i, repo_id) in variants.iter().enumerate() {
            let short_name = repo_id.rsplit('/').next().unwrap_or(repo_id);
            eprintln!("[{}/{}] {short_name}", i + 1, variants.len());
            snapshot_download.call1((repo_id.as_str(),))?;
        }

        Ok(())
    })
}

/// List all Qwen3-TTS models in the local HuggingFace cache.
///
/// Scans the HF cache directory and filters for repos matching
/// the `Qwen/Qwen3-TTS-12Hz-` or `mlx-community/Qwen3-TTS-12Hz-` prefix.
pub fn list_cached_models() -> Result<Vec<ModelInfo>, BridgeError> {
    Python::attach(|py| {
        let hf_hub = import_hf_hub(py)?;

        let cache_info = hf_hub.call_method0("scan_cache_dir")?;
        let repos = cache_info.getattr("repos")?;
        let repos_iter = PyIterator::from_object(&repos)?;

        let mut models = Vec::new();

        for repo in repos_iter {
            let repo: Bound<'_, PyAny> = repo?;
            let repo_id: String = repo.getattr("repo_id")?.extract()?;

            let is_qwen = repo_id.starts_with("Qwen/Qwen3-TTS-12Hz-")
                || repo_id.starts_with("mlx-community/Qwen3-TTS-12Hz-");
            let is_chatterbox = repo_id.starts_with("ResembleAI/chatterbox")
                || repo_id.starts_with("mlx-community/chatterbox");

            if !is_qwen && !is_chatterbox {
                continue;
            }

            let size_on_disk: u64 = repo.getattr("size_on_disk")?.extract()?;
            let repo_path: String = repo.getattr("repo_path")?.str()?.extract()?;

            // Extract a human-readable size label from the repo ID
            let label = if is_chatterbox {
                "ChatterBox"
            } else if repo_id.contains("1.7B") {
                "1.7B"
            } else {
                "-"
            };

            let (engine, variant_label) = if is_chatterbox {
                ("chatterbox".to_string(), chatterbox_variant_label(&repo_id))
            } else {
                ("qwen".to_string(), None)
            };

            models.push(ModelInfo {
                repo_id,
                size_label: label.to_string(),
                local_path: Some(repo_path),
                size_bytes: Some(size_on_disk),
                engine,
                variant_label,
            });
        }

        Ok(models)
    })
}

/// Download ChatterBox model variants from HuggingFace.
///
/// Downloads all ChatterBox model repos using `huggingface_hub.snapshot_download()`.
pub fn download_model_chatterbox() -> Result<(), BridgeError> {
    let variants = chatterbox_model_variants();

    Python::attach(|py| {
        let hf_hub = import_hf_hub(py)?;
        let snapshot_download = hf_hub.getattr("snapshot_download")?;

        for (i, repo_id) in variants.iter().enumerate() {
            let short_name = repo_id.rsplit('/').next().unwrap_or(repo_id);
            eprintln!("[{}/{}] {short_name}", i + 1, variants.len());
            snapshot_download.call1((repo_id.as_str(),))?;
        }

        Ok(())
    })
}

/// List all ChatterBox models in the local HuggingFace cache.
///
/// Scans the HF cache directory and filters for repos matching
/// the `ResembleAI/chatterbox` or `mlx-community/chatterbox` prefix.
pub fn list_cached_chatterbox_models() -> Result<Vec<ModelInfo>, BridgeError> {
    Python::attach(|py| {
        let hf_hub = import_hf_hub(py)?;

        let cache_info = hf_hub.call_method0("scan_cache_dir")?;
        let repos = cache_info.getattr("repos")?;
        let repos_iter = PyIterator::from_object(&repos)?;

        let mut models = Vec::new();

        for repo in repos_iter {
            let repo: Bound<'_, PyAny> = repo?;
            let repo_id: String = repo.getattr("repo_id")?.extract()?;

            if !repo_id.starts_with("ResembleAI/chatterbox")
                && !repo_id.starts_with("mlx-community/chatterbox")
            {
                continue;
            }

            let size_on_disk: u64 = repo.getattr("size_on_disk")?.extract()?;
            let repo_path: String = repo.getattr("repo_path")?.str()?.extract()?;

            models.push(ModelInfo {
                repo_id,
                size_label: "ChatterBox".to_string(),
                local_path: Some(repo_path),
                size_bytes: Some(size_on_disk),
                engine: "chatterbox".to_string(),
                variant_label: None,
            });
        }

        Ok(models)
    })
}

/// Remove all cached ChatterBox model variants.
///
/// Scans the HF cache and removes repos matching ChatterBox patterns.
pub fn remove_chatterbox_models() -> Result<(), BridgeError> {
    let variants = chatterbox_model_variants();

    Python::attach(|py| {
        let hf_hub = import_hf_hub(py)?;

        let cache_info = hf_hub.call_method0("scan_cache_dir")?;
        let repos = cache_info.getattr("repos")?;
        let repos_iter = PyIterator::from_object(&repos)?;

        let mut revision_hashes: Vec<String> = Vec::new();

        for repo in repos_iter {
            let repo: Bound<'_, PyAny> = repo?;
            let repo_id: String = repo.getattr("repo_id")?.extract()?;

            if !variants.iter().any(|v| v == &repo_id) {
                continue;
            }

            let revisions = repo.getattr("revisions")?;
            let revisions_iter = PyIterator::from_object(&revisions)?;
            for rev in revisions_iter {
                let rev: Bound<'_, PyAny> = rev?;
                let commit_hash: String = rev.getattr("commit_hash")?.extract()?;
                revision_hashes.push(commit_hash);
            }
        }

        if revision_hashes.is_empty() {
            return Err(BridgeError::ModelNotFound(
                "No cached ChatterBox models found".to_string(),
            ));
        }

        let delete_strategy = cache_info.call_method1("delete_revisions", (revision_hashes,))?;
        delete_strategy.call_method0("execute")?;

        Ok(())
    })
}

/// Check whether the `chatterbox-tts` Python package is installed in the venv.
///
/// Uses `importlib.metadata` (no heavy import) so it's safe to call from `model list`.
pub fn is_chatterbox_package_installed() -> bool {
    Python::attach(|py| {
        py.import("importlib.metadata")
            .ok()
            .and_then(|m| m.call_method1("version", ("chatterbox-tts",)).ok())
            .is_some()
    })
}

/// Remove all cached 1.7B model variants.
///
/// Detects the compute backend to determine which variant repos to remove.
/// Uses `huggingface_hub.scan_cache_dir()` to find matching revisions
/// and deletes them via the cache management API.
pub fn remove_model() -> Result<(), BridgeError> {
    let backend = super::runtime::detect_backend()?;
    // Remove all quantization variants (both bf16 and 8bit)
    let mut variants = model_variants(&backend, &ModelQuantization::Bf16);
    variants.extend(model_variants(&backend, &ModelQuantization::EightBit));

    Python::attach(|py| {
        let hf_hub = import_hf_hub(py)?;

        let cache_info = hf_hub.call_method0("scan_cache_dir")?;
        let repos = cache_info.getattr("repos")?;
        let repos_iter = PyIterator::from_object(&repos)?;

        let mut revision_hashes: Vec<String> = Vec::new();

        for repo in repos_iter {
            let repo: Bound<'_, PyAny> = repo?;
            let repo_id: String = repo.getattr("repo_id")?.extract()?;

            if !variants.iter().any(|v| v == &repo_id) {
                continue;
            }

            let revisions = repo.getattr("revisions")?;
            let revisions_iter = PyIterator::from_object(&revisions)?;
            for rev in revisions_iter {
                let rev: Bound<'_, PyAny> = rev?;
                let commit_hash: String = rev.getattr("commit_hash")?.extract()?;
                revision_hashes.push(commit_hash);
            }
        }

        if revision_hashes.is_empty() {
            return Err(BridgeError::ModelNotFound(format!(
                "No cached models found for size {}",
                size_label()
            )));
        }

        // Use the delete_revisions strategy from huggingface_hub
        let delete_strategy = cache_info.call_method1("delete_revisions", (revision_hashes,))?;
        delete_strategy.call_method0("execute")?;

        Ok(())
    })
}
