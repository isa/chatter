use std::path::Path;

use pyo3::prelude::*;

use super::error::BridgeError;

/// Run voice design inference. Returns (audio_samples_f32, sample_rate).
pub fn voice_design(
    text: &str,
    language: &str,
    instruct: &str,
) -> Result<(Vec<f32>, u32), BridgeError> {
    Python::attach(|py| {
        let bridge = import_bridge(py)?;
        let result = bridge.call_method1("voice_design", (text, language, instruct))?;
        let wav: Vec<f32> = result.get_item(0)?.extract()?;
        let sr: u32 = result.get_item(1)?.extract()?;
        Ok((wav, sr))
    })
}

/// Create a reusable voice clone prompt from reference audio.
/// Saves the prompt to voice_prompt.bin in the profile directory.
/// For MLX, saves ref_audio.wav instead (MLX has no prompt serialization).
pub fn create_and_save_clone_prompt(
    ref_audio_path: &Path,
    ref_text: &str,
    profile_dir: &Path,
) -> Result<(), BridgeError> {
    Python::attach(|py| {
        let bridge = import_bridge(py)?;
        let backend: String = bridge.call_method0("detect_backend")?.extract()?;

        if backend == "mlx" {
            // MLX: copy reference audio to profile as ref_audio.wav
            let dest = profile_dir.join("ref_audio.wav");
            std::fs::copy(ref_audio_path, &dest).map_err(|e| {
                BridgeError::VoiceCloneFailed(format!("Failed to copy ref audio: {e}"))
            })?;
        } else {
            // CUDA/MPS: create and save clone prompt
            let prompt = bridge.call_method1(
                "create_clone_prompt",
                (ref_audio_path.to_string_lossy().as_ref(), ref_text),
            )?;
            let prompt_path = profile_dir.join("voice_prompt.bin");
            bridge.call_method1(
                "save_clone_prompt",
                (prompt, prompt_path.to_string_lossy().as_ref()),
            )?;
        }
        Ok(())
    })
}

/// Generate speech from text using a saved profile.
/// Returns (audio_samples_f32, sample_rate).
pub fn generate_speech(
    text: &str,
    language: &str,
    profile_dir: &Path,
) -> Result<(Vec<f32>, u32), BridgeError> {
    Python::attach(|py| {
        let bridge = import_bridge(py)?;
        let result = bridge.call_method1(
            "generate_speech",
            (text, language, profile_dir.to_string_lossy().as_ref()),
        )?;
        let wav: Vec<f32> = result.get_item(0)?.extract()?;
        let sr: u32 = result.get_item(1)?.extract()?;
        Ok((wav, sr))
    })
}

/// Generate speech by cloning from a reference audio file directly.
/// Used during clone profile creation for the preview sample.
pub fn voice_clone_from_audio(
    ref_audio_path: &Path,
    text: &str,
    language: &str,
) -> Result<(Vec<f32>, u32), BridgeError> {
    Python::attach(|py| {
        let bridge = import_bridge(py)?;
        let result = bridge.call_method1(
            "voice_clone_from_audio",
            (ref_audio_path.to_string_lossy().as_ref(), text, language),
        )?;
        let wav: Vec<f32> = result.get_item(0)?.extract()?;
        let sr: u32 = result.get_item(1)?.extract()?;
        Ok((wav, sr))
    })
}

/// Release all cached Python models to free memory.
pub fn unload_all_models() -> Result<(), BridgeError> {
    Python::attach(|py| {
        let bridge = import_bridge(py)?;
        bridge.call_method0("unload_all_models")?;
        Ok(())
    })
}

/// Get the detected backend string from Python.
pub fn detected_backend() -> Result<String, BridgeError> {
    Python::attach(|py| {
        let bridge = import_bridge(py)?;
        let backend: String = bridge.call_method0("detect_backend")?.extract()?;
        Ok(backend)
    })
}

/// Import the chatter_bridge module, with a friendly error message.
fn import_bridge(py: Python<'_>) -> Result<Bound<'_, PyModule>, BridgeError> {
    py.import("chatter_bridge").map_err(|e| {
        if e.is_instance_of::<pyo3::exceptions::PyModuleNotFoundError>(py) {
            BridgeError::BackendNotAvailable(
                "chatter_bridge module not found. Run `chatter doctor` to verify setup."
                    .to_string(),
            )
        } else {
            BridgeError::Python(e)
        }
    })
}
