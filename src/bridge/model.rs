use pyo3::prelude::*;
use pyo3::types::PyIterator;

use crate::cli::ModelSize;

use super::error::BridgeError;

/// Information about a cached model.
pub struct ModelInfo {
    pub repo_id: String,
    pub size_label: String,
    pub local_path: Option<String>,
    pub size_bytes: Option<u64>,
}

/// All Qwen3-TTS model variants for a given size.
fn model_variants(size: &ModelSize) -> Vec<String> {
    match size {
        ModelSize::B1_7 => vec![
            "Qwen/Qwen3-TTS-12Hz-1.7B-VoiceDesign".to_string(),
            "Qwen/Qwen3-TTS-12Hz-1.7B-CustomVoice".to_string(),
            "Qwen/Qwen3-TTS-12Hz-1.7B-Base".to_string(),
        ],
        ModelSize::B0_6 => vec![
            "Qwen/Qwen3-TTS-12Hz-0.6B-CustomVoice".to_string(),
            "Qwen/Qwen3-TTS-12Hz-0.6B-Base".to_string(),
        ],
    }
}

/// Return a human-readable label for the model size.
pub fn size_label(size: &ModelSize) -> &'static str {
    match size {
        ModelSize::B1_7 => "1.7B",
        ModelSize::B0_6 => "0.6B",
    }
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

/// Download all model variants for the given size from HuggingFace.
///
/// Uses `huggingface_hub.snapshot_download()` which downloads model files
/// to the default HF cache (`~/.cache/huggingface/hub/`).
pub fn download_model(size: &ModelSize) -> Result<(), BridgeError> {
    let variants = model_variants(size);

    Python::attach(|py| {
        let hf_hub = import_hf_hub(py)?;
        let snapshot_download = hf_hub.getattr("snapshot_download")?;

        for repo_id in &variants {
            snapshot_download.call1((repo_id.as_str(),))?;
        }

        Ok(())
    })
}

/// List all Qwen3-TTS models in the local HuggingFace cache.
///
/// Scans the HF cache directory and filters for repos matching
/// the `Qwen/Qwen3-TTS-12Hz-` prefix.
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

            if !repo_id.starts_with("Qwen/Qwen3-TTS-12Hz-") {
                continue;
            }

            let size_on_disk: u64 = repo.getattr("size_on_disk")?.extract()?;
            let repo_path: String = repo.getattr("repo_path")?.str()?.extract()?;

            // Extract a human-readable size label from the repo ID
            let label = if repo_id.contains("0.6B") {
                "0.6B"
            } else if repo_id.contains("1.7B") {
                "1.7B"
            } else {
                "unknown"
            };

            models.push(ModelInfo {
                repo_id,
                size_label: label.to_string(),
                local_path: Some(repo_path),
                size_bytes: Some(size_on_disk),
            });
        }

        Ok(models)
    })
}

/// Remove all cached model variants for the given size.
///
/// Uses `huggingface_hub.scan_cache_dir()` to find matching revisions
/// and deletes them via the cache management API.
pub fn remove_model(size: &ModelSize) -> Result<(), BridgeError> {
    let variants = model_variants(size);

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
                size_label(size)
            )));
        }

        // Use the delete_revisions strategy from huggingface_hub
        let delete_strategy = cache_info.call_method1(
            "delete_revisions",
            (revision_hashes,),
        )?;
        delete_strategy.call_method0("execute")?;

        Ok(())
    })
}
