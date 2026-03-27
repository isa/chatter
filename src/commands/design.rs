use std::fs;
use std::io::{self, BufRead, Write};

use anyhow::Context;

use crate::audio;
use crate::bridge::inference;
use crate::cli::{DesignArgs, GlobalArgs, Language};
use crate::profile::storage::{self, PREVIEW_SENTENCE};
use crate::profile::{AudioInfo, ProfileInfo, ProfileMetadata, ProfileType};
use crate::ui;

/// Map CLI Language enum to the string expected by the Python bridge.
fn language_to_str(lang: &Language) -> &'static str {
    match lang {
        Language::Auto => "auto",
        Language::Chinese => "zh",
        Language::English => "en",
        Language::Japanese => "ja",
        Language::Korean => "ko",
        Language::French => "fr",
        Language::German => "de",
        Language::Spanish => "es",
        Language::Portuguese => "pt",
        Language::Russian => "ru",
        Language::Italian => "it",
    }
}

pub fn run(args: DesignArgs, global: &GlobalArgs) -> anyhow::Result<()> {
    // 1. Resolve profile name
    let name = match &args.name {
        Some(n) => n.clone(),
        None => {
            let slug = storage::slugify(&args.description, 4);
            storage::unique_profile_name(&slug)?
        }
    };

    // 2. Resolve language string for Python bridge
    let language_str = language_to_str(&global.language);

    // 3. Create profile directory early (needed for temp files)
    let profile_dir = storage::profile_dir(&name)?;
    fs::create_dir_all(&profile_dir)
        .with_context(|| format!("Failed to create profile directory: {}", profile_dir.display()))?;

    let mut description = args.description.clone();
    // Interactive design loop
    let (wav, sr) = loop {
        // Show spinner during inference
        let spinner = ui::create_spinner("Loading VoiceDesign model and designing voice...");

        let result = inference::voice_design(PREVIEW_SENTENCE, language_str, &description);

        // Finish spinner before handling result
        let (wav, sr) = match result {
            Ok(data) => {
                spinner.finish_and_clear();
                data
            }
            Err(e) => {
                spinner.finish_and_clear();
                // Clean up profile dir if we created it
                let _ = fs::remove_dir_all(&profile_dir);
                return Err(anyhow::anyhow!(e).context("Voice design inference failed"));
            }
        };

        // Encode preview to temp MP3
        let temp_mp3 = profile_dir.join("preview_tmp.mp3");
        let pcm = audio::samples_f32_to_i16(&wav);
        audio::encode_wav_to_mp3(&pcm, sr, &temp_mp3)
            .context("Failed to encode preview audio")?;

        eprintln!("Preview of your custom voice:");

        // Play preview audio
        audio::playback::play_audio(&temp_mp3)
            .context("Failed to play preview audio")?;

        // Clean up temp preview
        let _ = fs::remove_file(&temp_mp3);

        // Interactive accept/retry prompt
        eprint!("Accept this voice? [Y/n/new description] ");
        io::stderr().flush()?;

        let mut input = String::new();
        let stdin = io::stdin();
        stdin.lock().read_line(&mut input)?;

        match input.trim() {
            "" | "y" | "Y" | "yes" => {
                break (wav, sr);
            }
            "n" | "N" | "no" => {
                // Clean up profile dir
                let _ = fs::remove_dir_all(&profile_dir);
                anyhow::bail!("Voice design cancelled by user");
            }
            new_desc => {
                description = new_desc.to_string();
                // Loop back with new description (model stays cached in Python)
            }
        }
    };

    // Save WAV to profile dir (needed for clone prompt creation)
    let ref_wav_path = profile_dir.join("ref_audio.wav");
    {
        let spec = hound::WavSpec {
            channels: 1,
            sample_rate: sr,
            bits_per_sample: 32,
            sample_format: hound::SampleFormat::Float,
        };
        let mut writer = hound::WavWriter::create(&ref_wav_path, spec)
            .context("Failed to create WAV file for clone prompt")?;
        for &sample in &wav {
            writer
                .write_sample(sample)
                .context("Failed to write WAV sample")?;
        }
        writer.finalize().context("Failed to finalize WAV file")?;
    }

    // Create reusable clone prompt (saves voice_prompt.bin or keeps ref_audio.wav for MLX)
    inference::create_and_save_clone_prompt(&ref_wav_path, PREVIEW_SENTENCE, &profile_dir)
        .map_err(|e| anyhow::anyhow!(e).context("Failed to create clone prompt"))?;

    // Encode accepted audio to sample.mp3
    let sample_mp3_path = profile_dir.join("sample.mp3");
    let pcm = audio::samples_f32_to_i16(&wav);
    audio::encode_wav_to_mp3(&pcm, sr, &sample_mp3_path)
        .context("Failed to encode sample MP3")?;

    // Determine model variant based on detected backend
    let backend = inference::detected_backend()
        .map_err(|e| anyhow::anyhow!(e).context("Failed to detect backend"))?;
    let model_variant = if backend == "mlx" {
        "mlx-community/Qwen3-TTS-12Hz-1.7B-VoiceDesign-bf16".to_string()
    } else {
        "Qwen/Qwen3-TTS-12Hz-1.7B-VoiceDesign".to_string()
    };

    // Build and save profile metadata
    let metadata = ProfileMetadata {
        profile: ProfileInfo {
            name: name.clone(),
            profile_type: ProfileType::Designed,
            language: language_str.to_string(),
            description: Some(args.description.clone()),
            source_audio: None,
            created: chrono::Utc::now().to_rfc3339(),
            model_variant,
        },
        audio: AudioInfo {
            sample_text: PREVIEW_SENTENCE.to_string(),
            sample_rate: sr,
        },
    };

    storage::save_profile(&metadata).context("Failed to save profile metadata")?;

    // Unload models to free memory
    let _ = inference::unload_all_models();

    eprintln!(
        "Voice profile '{}' saved to {}/",
        name,
        profile_dir.display()
    );
    eprintln!("  Sample: {}", sample_mp3_path.display());

    Ok(())
}
