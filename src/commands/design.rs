use std::fs;

use anyhow::Context;
use dialoguer::{Input, Select};

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
    let language_str = language_to_str(&global.language);
    let mut description = args.description.clone();
    let mut attempt = 0u32;

    // Use system temp dir for previews during the design loop
    let temp_dir = std::env::temp_dir().join("chatter-preview");
    fs::create_dir_all(&temp_dir)?;

    // Interactive design loop
    let (wav, sr) = loop {
        attempt += 1;

        // First attempt: load model without spinner so Python's download/checkpoint
        // progress shows through. Subsequent attempts: model is cached, use spinner.
        if attempt == 1 {
            let was_cached = inference::ensure_model_loaded("design")
                .map_err(|e| anyhow::anyhow!(e).context("Failed to load VoiceDesign model"))?;
            if !was_cached {
                eprintln!();
            }
        }

        let spinner = ui::create_spinner(if attempt == 1 {
            "Designing voice..."
        } else {
            "Designing voice..."
        });

        let result = inference::voice_design(PREVIEW_SENTENCE, language_str, &description);

        let (wav, sr) = match result {
            Ok(data) => {
                spinner.finish_and_clear();
                data
            }
            Err(e) => {
                spinner.finish_and_clear();
                let _ = fs::remove_dir_all(&temp_dir);
                return Err(anyhow::anyhow!(e).context("Voice design inference failed"));
            }
        };

        // Encode preview to temp MP3
        let temp_mp3 = temp_dir.join("preview.mp3");
        let pcm = audio::samples_f32_to_i16(&wav);
        audio::encode_wav_to_mp3(&pcm, sr, &temp_mp3)
            .context("Failed to encode preview audio")?;

        eprintln!("\n  Playing preview...\n");
        audio::playback::play_audio(&temp_mp3)
            .context("Failed to play preview audio")?;
        let _ = fs::remove_file(&temp_mp3);

        // Interactive menu
        let choices = &[
            "Yes, accept this voice",
            "No, retry with same description",
            "Change the description",
            "Quit",
        ];

        let selection = Select::new()
            .with_prompt("What do you think?")
            .items(choices)
            .default(0)
            .interact()?;

        match selection {
            0 => break (wav, sr),
            1 => { /* retry with same description */ }
            2 => {
                let new_desc: String = Input::new()
                    .with_prompt("New voice description")
                    .with_initial_text(&description)
                    .interact_text()?;
                if !new_desc.trim().is_empty() {
                    description = new_desc.trim().to_string();
                }
            }
            3 => {
                let _ = fs::remove_dir_all(&temp_dir);
                anyhow::bail!("Voice design cancelled by user");
            }
            _ => unreachable!(),
        }
    };

    // Clean up temp dir
    let _ = fs::remove_dir_all(&temp_dir);

    // Prompt for profile name (use --name if provided, otherwise ask interactively)
    let default_name = match &args.name {
        Some(n) => n.clone(),
        None => {
            let slug = storage::slugify(&description, 4);
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
    let spinner = ui::create_spinner("Saving voice profile...");
    inference::create_and_save_clone_prompt(&ref_wav_path, PREVIEW_SENTENCE, &profile_dir)
        .map_err(|e| anyhow::anyhow!(e).context("Failed to create clone prompt"))?;

    // Encode accepted audio to sample.mp3
    let sample_mp3_path = profile_dir.join("sample.mp3");
    let pcm = audio::samples_f32_to_i16(&wav);
    audio::encode_wav_to_mp3(&pcm, sr, &sample_mp3_path)
        .context("Failed to encode sample MP3")?;
    spinner.finish_and_clear();

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
            description: Some(description.clone()),
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

    // Print summary box
    print_summary(&name, &description, language_str, attempt, &sample_mp3_path, &profile_dir);

    Ok(())
}

fn print_summary(
    name: &str,
    description: &str,
    language: &str,
    attempts: u32,
    sample_path: &std::path::Path,
    profile_dir: &std::path::Path,
) {
    let home = std::env::var("HOME").unwrap_or_default();
    let shorten = |s: &str| -> String {
        if !home.is_empty() && s.starts_with(&home) {
            format!("~{}", &s[home.len()..])
        } else {
            s.to_string()
        }
    };

    let profile_str = shorten(&format!("{}/", profile_dir.display()));
    let sample_str = shorten(&format!("{}", sample_path.display()));
    let usage_str = format!("chatter generate \"text\" --profile {name}");
    let title = format!("✓ Voice Profile Created: {name}");

    let rows: Vec<(&str, String)> = vec![
        ("Description", description.to_string()),
        ("Language", language.to_string()),
        ("Attempts", attempts.to_string()),
        ("Profile", profile_str),
        ("Sample", sample_str),
        ("Usage", usage_str),
    ];

    let lw = 14;
    let val_width = rows.iter().map(|(_, v)| v.len()).max().unwrap_or(30);
    let title_width = title.len();
    let inner = (lw + val_width).max(title_width) + 2;
    let bar = "─".repeat(inner);
    let vw = inner - lw - 2;

    eprintln!();
    eprintln!("  ╭{}╮", bar);
    eprintln!("  │ {:<w$} │", title, w = inner - 2);
    eprintln!("  ├{}┤", bar);
    for (label, value) in &rows[..3] {
        eprintln!("  │ {:<lw$}{:<vw$} │", format!("{label}:"), value);
    }
    eprintln!("  ├{}┤", bar);
    for (label, value) in &rows[3..5] {
        eprintln!("  │ {:<lw$}{:<vw$} │", format!("{label}:"), value);
    }
    eprintln!("  ├{}┤", bar);
    let (label, value) = &rows[5];
    eprintln!("  │ {:<lw$}{:<vw$} │", format!("{label}:"), value);
    eprintln!("  ╰{}╯", bar);
    eprintln!();
}
