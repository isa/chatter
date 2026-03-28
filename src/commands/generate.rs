use std::fs;
use std::path::PathBuf;

use anyhow::Context;
use owo_colors::{OwoColorize, Stream, Style};

use crate::audio;
use crate::bridge::inference;
use crate::cli::{GenerateArgs, GlobalArgs, Language};
use crate::profile::storage;
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

/// Format a byte count as a human-readable size string (KB or MB).
fn format_file_size(bytes: u64) -> String {
    if bytes >= 1_000_000 {
        format!("{:.1} MB", bytes as f64 / 1_000_000.0)
    } else {
        format!("{:.1} KB", bytes as f64 / 1_000.0)
    }
}

pub fn run(args: GenerateArgs, global: &GlobalArgs) -> anyhow::Result<()> {
    // 1. Get text input -- only inline text supported in Phase 2
    let text = match (&args.text, &args.file) {
        (Some(t), _) => t.clone(),
        (None, Some(_)) => {
            anyhow::bail!(
                "File input is not yet supported. It will be available in a future update."
            );
        }
        (None, None) => {
            anyhow::bail!(
                "Provide text to speak, e.g.: chatter generate \"Hello world\" --profile myvoice"
            );
        }
    };

    // 2. Load profile
    let profile = storage::load_profile(&args.profile).map_err(|_| {
        anyhow::anyhow!(
            "Profile '{}' not found. Run `chatter profiles list` to see available profiles.",
            args.profile
        )
    })?;

    // 3. Get profile directory
    let profile_dir = storage::profile_dir(&args.profile)?;

    // Verify profile has voice data
    let has_prompt = profile_dir.join("voice_prompt.bin").exists();
    let has_ref_audio = profile_dir.join("ref_audio.wav").exists();
    if !has_prompt && !has_ref_audio {
        anyhow::bail!(
            "Profile '{}' is missing voice data. Try recreating it with `chatter design` or `chatter clone`.",
            args.profile
        );
    }

    // 4. Resolve language per GEN-06: CLI flag overrides profile default
    let language_str = if global.language != Language::Auto {
        language_to_str(&global.language)
    } else {
        // Use the language stored in the profile
        // The profile stores language codes like "auto", "zh", "en", etc.
        // We leak a string here to get a &str with appropriate lifetime.
        // Since this runs once per CLI invocation, the leak is negligible.
        Box::leak(profile.profile.language.clone().into_boxed_str()) as &str
    };

    // 5. Resolve output path per D-15
    let output_path = match &args.output {
        Some(p) => p.clone(),
        None => {
            let timestamp = chrono::Local::now().format("%Y%m%d-%H%M%S");
            let filename = format!("{}-{}.mp3", profile.profile.name, timestamp);
            PathBuf::from(&filename)
        }
    };

    // 6. Check if output file exists per D-16
    if output_path.exists() {
        let warn_style = Style::new().yellow().bold();
        eprintln!(
            "{} Overwriting existing file: {}",
            "Warning:".if_supports_color(Stream::Stderr, |t| t.style(warn_style)),
            output_path.display()
        );
    }

    // 7. Show spinner per D-17 and UX-02
    let spinner = ui::create_spinner("Generating speech...");

    // 8. Call inference
    let (wav, sr) = inference::generate_speech(&text, language_str, &profile_dir)
        .map_err(|e| anyhow::anyhow!(e))
        .context("Speech generation failed")?;

    // 9. Finish spinner
    spinner.finish_and_clear();

    // 10. Encode to MP3 per GEN-05
    let pcm = audio::samples_f32_to_i16(&wav);
    audio::encode_wav_to_mp3(&pcm, sr, &output_path).context("MP3 encoding failed")?;

    // 11. Unload models to free memory
    let _ = inference::unload_all_models();

    // 12. Print success with file size
    let file_size = fs::metadata(&output_path)
        .map(|m| format_file_size(m.len()))
        .unwrap_or_else(|_| "unknown size".to_string());
    eprintln!("Generated: {} ({})", output_path.display(), file_size);

    // 13. Play audio per D-18
    if args.play {
        audio::playback::play_audio(&output_path).context("Audio playback failed")?;
    }

    Ok(())
}
