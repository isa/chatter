use std::fs;
use std::path::Path;

use anyhow::{bail, Context};
use chrono::Utc;
use dialoguer::{Input, Select};
use owo_colors::Style;

use crate::audio;
use crate::bridge::inference as bridge;
use crate::cli::{CloneArgs, GlobalArgs, Language};
use crate::profile::storage::{self, PREVIEW_SENTENCE};
use crate::profile::{AudioInfo, ProfileInfo, ProfileMetadata, ProfileType};
use crate::ui;

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

pub fn run(args: CloneArgs, global: &GlobalArgs) -> anyhow::Result<()> {
    // 1. Validate input file
    validate_audio_file(&args.audio_file)?;

    let language_str = language_to_string(&global.language);

    // Use system temp dir for previews during the clone loop
    let temp_dir = std::env::temp_dir().join("chatter-preview");
    fs::create_dir_all(&temp_dir)?;

    let mut attempt = 0u32;

    // Interactive clone loop
    let (wav, sr) = loop {
        attempt += 1;

        // First attempt: load model without spinner so Python's download/checkpoint
        // progress shows through. Subsequent attempts: model is cached, use spinner.
        if attempt == 1 {
            let _was_cached = bridge::ensure_model_loaded("custom")
                .map_err(|e| anyhow::anyhow!(e).context("Failed to load speech model"))?;
        }

        let spinner = ui::create_spinner("Cloning voice from reference audio...");
        let result =
            bridge::voice_clone_from_audio(&args.audio_file, PREVIEW_SENTENCE, &language_str);

        let (wav, sr) = match result {
            Ok(data) => {
                ui::finish_spinner(&spinner, "Voice cloned");
                data
            }
            Err(e) => {
                spinner.finish_and_clear();
                let _ = fs::remove_dir_all(&temp_dir);
                return Err(anyhow::anyhow!(e).context("Voice cloning failed"));
            }
        };

        // Encode preview to temp MP3
        let temp_mp3 = temp_dir.join("preview.mp3");
        let pcm = audio::samples_f32_to_i16(&wav);
        audio::encode_wav_to_mp3(&pcm, sr, &temp_mp3)
            .context("Failed to encode preview audio")?;

        let play_spinner = ui::create_spinner("Playing preview...");
        audio::playback::play_audio(&temp_mp3).context("Failed to play preview audio")?;
        ui::finish_spinner(&play_spinner, "Preview played");
        let _ = fs::remove_file(&temp_mp3);

        // Interactive menu
        let choices = &[
            "Yes, accept this voice",
            "No, retry",
            "Quit",
        ];

        let selection = Select::new()
            .with_prompt("What do you think?")
            .items(choices)
            .default(0)
            .interact()?;

        match selection {
            0 => break (wav, sr),
            1 => { /* retry */ }
            2 => {
                let _ = fs::remove_dir_all(&temp_dir);
                anyhow::bail!("Voice cloning cancelled by user");
            }
            _ => unreachable!(),
        }
    };

    // Clean up temp dir
    let _ = fs::remove_dir_all(&temp_dir);

    // Prompt for profile name
    let default_name = match &args.name {
        Some(n) => n.clone(),
        None => {
            let stem = args
                .audio_file
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("clone");
            let slug = storage::slugify(stem, 4);
            storage::unique_profile_name(&slug)?
        }
    };

    let name: String = if args.name.is_some() {
        default_name
    } else {
        Input::new()
            .with_prompt("Profile name")
            .default(default_name)
            .interact_text()?
            .trim()
            .to_string()
    };

    if name.is_empty() {
        anyhow::bail!("Profile name cannot be empty");
    }

    // Create profile directory
    let profile_dir = storage::profile_dir(&name)?;
    fs::create_dir_all(&profile_dir)
        .with_context(|| format!("Failed to create profile directory: {}", profile_dir.display()))?;

    // Save clone prompt + reference audio + sample MP3
    let spinner = ui::create_spinner("Saving voice profile...");

    bridge::create_and_save_clone_prompt(&args.audio_file, PREVIEW_SENTENCE, &profile_dir)
        .map_err(|e| anyhow::anyhow!(e).context("Failed to save clone prompt"))?;

    // Save WAV to profile dir (needed for MLX which uses ref_audio directly)
    let ref_wav_path = profile_dir.join("ref_audio.wav");
    {
        let spec = hound::WavSpec {
            channels: 1,
            sample_rate: sr,
            bits_per_sample: 32,
            sample_format: hound::SampleFormat::Float,
        };
        let mut writer = hound::WavWriter::create(&ref_wav_path, spec)
            .context("Failed to create WAV file")?;
        for &sample in &wav {
            writer
                .write_sample(sample)
                .context("Failed to write WAV sample")?;
        }
        writer.finalize().context("Failed to finalize WAV file")?;
    }

    // Encode accepted audio to sample.mp3
    let sample_mp3_path = profile_dir.join("sample.mp3");
    let pcm = audio::samples_f32_to_i16(&wav);
    audio::encode_wav_to_mp3(&pcm, sr, &sample_mp3_path)
        .context("Failed to encode sample MP3")?;
    ui::finish_spinner(&spinner, "Voice profile saved");

    // Determine model variant based on detected backend
    let backend = bridge::detected_backend().unwrap_or_else(|_| "unknown".to_string());
    let model_variant = match backend.as_str() {
        "mlx" => "mlx-community/Qwen3-TTS-12Hz-1.7B-Base-bf16".to_string(),
        _ => "Qwen/Qwen3-TTS-1.7B-CustomVoice".to_string(),
    };

    // Build and save profile metadata
    let source_audio = args
        .audio_file
        .canonicalize()
        .unwrap_or_else(|_| args.audio_file.clone())
        .to_string_lossy()
        .to_string();

    let metadata = ProfileMetadata {
        profile: ProfileInfo {
            name: name.clone(),
            profile_type: ProfileType::Cloned,
            language: language_str.clone(),
            description: None,
            source_audio: Some(source_audio.clone()),
            created: Utc::now().to_rfc3339(),
            model_variant,
        },
        audio: AudioInfo {
            sample_text: PREVIEW_SENTENCE.to_string(),
            sample_rate: sr,
        },
    };

    storage::save_profile(&metadata).context("Failed to save profile metadata")?;

    // Unload models to free memory
    let _ = bridge::unload_all_models();

    // Print summary box
    print_summary(
        &name,
        &source_audio,
        &language_str,
        attempt,
        &sample_mp3_path,
        &profile_dir,
    );

    Ok(())
}

/// Validate that the input audio file exists and has an acceptable format.
fn validate_audio_file(path: &Path) -> anyhow::Result<()> {
    if !path.exists() {
        bail!("File not found: {}", path.display());
    }

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

    let file_size = fs::metadata(path)
        .context("Cannot read file metadata")?
        .len();

    if file_size == 0 {
        bail!("File is empty: {}", path.display());
    }

    if ext_str == "wav" {
        validate_wav(path)?;
    } else if file_size < 1000 {
        bail!(
            "File is too small ({} bytes) to be a valid MP3: {}",
            file_size,
            path.display()
        );
    }

    Ok(())
}

/// Validate WAV file properties and warn about unusual parameters.
fn validate_wav(path: &Path) -> anyhow::Result<()> {
    use owo_colors::{OwoColorize, Stream};

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

fn print_summary(
    name: &str,
    source_audio: &str,
    language: &str,
    attempts: u32,
    sample_path: &std::path::Path,
    profile_dir: &std::path::Path,
) {
    let title = format!("\u{2714} Voice Profile Cloned: {name}");
    ui::print_summary_box(
        &title,
        &[
            ui::SummarySection {
                rows: vec![
                    ("Source", source_audio.to_string(), false),
                    ("Language", language.to_string(), false),
                    ("Attempts", attempts.to_string(), false),
                ],
            },
            ui::SummarySection {
                rows: vec![
                    ("Profile", format!("{}/", profile_dir.display()), true),
                    ("Sample", format!("{}", sample_path.display()), true),
                ],
            },
            ui::SummarySection {
                rows: vec![(
                    "Usage",
                    format!("chatter generate \"text\" --profile {name}"),
                    false,
                )],
            },
        ],
    );
}
