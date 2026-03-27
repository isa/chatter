use std::fs;
use std::path::Path;

use anyhow::{bail, Context};
use chrono::Utc;
use owo_colors::{OwoColorize, Stream, Style};

use crate::audio;
use crate::bridge::inference as bridge;
use crate::cli::{CloneArgs, GlobalArgs, Language};
use crate::profile::storage::{self, PREVIEW_SENTENCE};
use crate::profile::{AudioInfo, ProfileInfo, ProfileMetadata, ProfileType};
use crate::ui;

pub fn run(args: CloneArgs, global: &GlobalArgs) -> anyhow::Result<()> {
    // 1. Validate input file
    validate_audio_file(&args.audio_file)?;

    // 2. Resolve profile name
    let name = resolve_profile_name(&args)?;

    // 3. Resolve language
    let language_str = language_to_string(&global.language);

    // 4. Show spinner
    let spinner = ui::create_spinner("Cloning voice from reference audio...");

    // 5. Generate preview sample using reference audio directly
    let (wav, sr) = match bridge::voice_clone_from_audio(&args.audio_file, PREVIEW_SENTENCE, &language_str) {
        Ok(result) => result,
        Err(e) => {
            spinner.finish_and_clear();
            ui::print_error(
                "Voice cloning failed.",
                Some(&format!("{e:#}")),
                global.verbose,
            );
            return Err(e.into());
        }
    };

    // 6. Create profile directory
    let profile_dir = storage::profile_dir(&name)?;
    fs::create_dir_all(&profile_dir)
        .context("Failed to create profile directory")?;

    // 7. Save clone prompt (voice_prompt.bin on CUDA/MPS, ref_audio.wav on MLX)
    if let Err(e) = bridge::create_and_save_clone_prompt(&args.audio_file, PREVIEW_SENTENCE, &profile_dir) {
        spinner.finish_and_clear();
        ui::print_error(
            "Failed to save clone prompt.",
            Some(&format!("{e:#}")),
            global.verbose,
        );
        return Err(e.into());
    }

    // 8. Encode preview to sample.mp3
    let pcm = audio::samples_f32_to_i16(&wav);
    audio::encode_wav_to_mp3(&pcm, sr, &profile_dir.join("sample.mp3"))?;

    // 9. Finish spinner
    spinner.finish_and_clear();

    // 10. Build and save ProfileMetadata
    let backend = bridge::detected_backend().unwrap_or_else(|_| "unknown".to_string());
    let model_variant = match backend.as_str() {
        "mlx" => "mlx-community/Qwen3-TTS-0.6B-bf16".to_string(),
        _ => "Qwen/Qwen3-TTS-1.7B-CustomVoice".to_string(),
    };

    let metadata = ProfileMetadata {
        profile: ProfileInfo {
            name: name.clone(),
            profile_type: ProfileType::Cloned,
            language: language_str,
            description: None,
            source_audio: Some(
                args.audio_file
                    .canonicalize()
                    .unwrap_or_else(|_| args.audio_file.clone())
                    .to_string_lossy()
                    .to_string(),
            ),
            created: Utc::now().to_rfc3339(),
            model_variant,
        },
        audio: AudioInfo {
            sample_text: PREVIEW_SENTENCE.to_string(),
            sample_rate: sr,
        },
    };

    // 11. Save profile
    storage::save_profile(&metadata)?;

    // 12. Unload models to free memory
    let _ = bridge::unload_all_models();

    // 13. Print success
    let sample_path = profile_dir.join("sample.mp3");
    let success_style = Style::new().green().bold();
    eprintln!(
        "\n{} Voice profile '{}' saved.",
        "Done:".if_supports_color(Stream::Stderr, |t| t.style(success_style)),
        name
    );
    eprintln!("Sample audio: {}", sample_path.display());

    Ok(())
}

/// Validate that the input audio file exists and has an acceptable format.
fn validate_audio_file(path: &Path) -> anyhow::Result<()> {
    // Check existence
    if !path.exists() {
        bail!("File not found: {}", path.display());
    }

    // Check extension
    let ext = path
        .extension()
        .map(|e| e.to_ascii_lowercase())
        .unwrap_or_default();
    let ext_str = ext.to_string_lossy();

    if ext_str != "mp3" && ext_str != "wav" {
        bail!(
            "Unsupported audio format '.{}'. Chatter accepts MP3 or WAV files.",
            ext_str
        );
    }

    // Check non-zero size
    let file_size = fs::metadata(path)
        .context("Cannot read file metadata")?
        .len();

    if file_size == 0 {
        bail!("File is empty: {}", path.display());
    }

    // Format-specific validation
    if ext_str == "wav" {
        validate_wav(path)?;
    } else {
        // MP3: basic sanity check on file size
        if file_size < 1000 {
            bail!(
                "File is too small ({} bytes) to be a valid MP3: {}",
                file_size,
                path.display()
            );
        }
    }

    Ok(())
}

/// Validate WAV file properties and warn about unusual parameters.
fn validate_wav(path: &Path) -> anyhow::Result<()> {
    let reader = hound::WavReader::open(path)
        .context("Failed to read WAV file. Is the file corrupted?")?;

    let spec = reader.spec();
    let total_samples = reader.len() as f64;
    let duration = total_samples / (spec.channels as f64 * spec.sample_rate as f64);

    if duration < 1.0 || duration > 30.0 {
        let warn_style = Style::new().yellow();
        eprintln!(
            "{} Reference audio is {:.1}s. Best results are with 3-10 seconds of clean speech.",
            "Warning:".if_supports_color(Stream::Stderr, |t| t.style(warn_style)),
            duration
        );
    }

    let sr = spec.sample_rate;
    if sr != 16000 && sr != 24000 && sr != 44100 && sr != 48000 {
        let warn_style = Style::new().yellow();
        eprintln!(
            "{} Unusual sample rate ({}Hz). Best results with 16kHz-48kHz.",
            "Warning:".if_supports_color(Stream::Stderr, |t| t.style(warn_style)),
            sr
        );
    }

    Ok(())
}

/// Resolve profile name from --name flag or audio filename.
fn resolve_profile_name(args: &CloneArgs) -> anyhow::Result<String> {
    let base = match &args.name {
        Some(name) => name.clone(),
        None => {
            let stem = args
                .audio_file
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("clone");
            storage::slugify(stem, 4)
        }
    };
    storage::unique_profile_name(&base)
}

/// Map Language enum to the string expected by the Python bridge.
fn language_to_string(lang: &Language) -> String {
    match lang {
        Language::Auto => "auto".to_string(),
        Language::Chinese => "Chinese".to_string(),
        Language::English => "English".to_string(),
        Language::Japanese => "Japanese".to_string(),
        Language::Korean => "Korean".to_string(),
        Language::French => "French".to_string(),
        Language::German => "German".to_string(),
        Language::Spanish => "Spanish".to_string(),
        Language::Portuguese => "Portuguese".to_string(),
        Language::Russian => "Russian".to_string(),
        Language::Italian => "Italian".to_string(),
    }
}
