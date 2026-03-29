use std::fs;
use std::path::{Path, PathBuf};

use anyhow::Context;
use owo_colors::{OwoColorize, Stream, Style};

use crate::audio;
use crate::bridge::inference;
use crate::bridge::model::ModelQuantization;
use crate::chunk;
use crate::cli::{Engine, GenerateArgs, GlobalArgs, Language};
use crate::extract;
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

/// Generate zero-valued f32 samples for silence gaps between chunks.
fn silence_samples(duration_ms: u32, sample_rate: u32) -> Vec<f32> {
    vec![0.0f32; ((sample_rate as u64 * duration_ms as u64) / 1000) as usize]
}

/// Concatenate audio chunks with silence gaps between them.
/// All chunks must have the same sample rate.
fn concatenate_chunks(chunks: &[(Vec<f32>, u32)], gap_ms: u32) -> (Vec<f32>, u32) {
    assert!(!chunks.is_empty(), "Cannot concatenate empty chunks");
    let sample_rate = chunks[0].1;
    // Verify all sample rates match
    for (i, (_, sr)) in chunks.iter().enumerate().skip(1) {
        assert_eq!(
            *sr, sample_rate,
            "Sample rate mismatch: chunk 0 has {sample_rate}, chunk {i} has {sr}"
        );
    }

    let gap = silence_samples(gap_ms, sample_rate);
    let total_len: usize =
        chunks.iter().map(|(s, _)| s.len()).sum::<usize>() + gap.len() * (chunks.len() - 1);
    let mut combined = Vec::with_capacity(total_len);

    for (i, (samples, _)) in chunks.iter().enumerate() {
        if i > 0 {
            combined.extend_from_slice(&gap);
        }
        combined.extend_from_slice(samples);
    }

    (combined, sample_rate)
}

/// Generate a split output path with 3-digit zero-padded index.
/// e.g., "foo.mp3" + index 1 -> "foo-001.mp3"
fn split_output_path(base: &Path, index: usize) -> PathBuf {
    let stem = base
        .file_stem()
        .unwrap_or_default()
        .to_string_lossy();
    let ext = base
        .extension()
        .unwrap_or_default()
        .to_string_lossy();
    let filename = format!("{stem}-{index:03}.{ext}");
    match base.parent() {
        Some(parent) if parent != Path::new("") => parent.join(filename),
        _ => PathBuf::from(filename),
    }
}

pub fn run(args: GenerateArgs, global: &GlobalArgs) -> anyhow::Result<()> {
    let quant_override = args.variant.map(ModelQuantization::from);

    // 1. Get text input -- inline text or file extraction
    let text = match (&args.text, &args.file) {
        (Some(t), _) => t.clone(),
        (None, Some(file_path)) => {
            // D-11: "Reading file..." spinner
            let spinner = ui::create_spinner("Reading file...");
            let raw = extract::extract_text(file_path)
                .context(format!("Failed to process file: {}", file_path.display()))?;
            spinner.finish_and_clear();
            if raw.trim().is_empty() {
                anyhow::bail!("No text content found in file: {}", file_path.display());
            }
            raw
        }
        (None, None) => {
            anyhow::bail!(
                "Provide text or a file to speak.\n\n\
                 Examples:\n  \
                 chatter generate \"Hello world\" --profile myvoice\n  \
                 chatter generate --file document.pdf --profile myvoice\n  \
                 chatter generate --file notes.md --profile myvoice --split"
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

    // 2b. Validate engine matches profile
    let profile_engine = &profile.profile.engine;
    let cli_engine = global.engine.as_str();
    if profile_engine != cli_engine {
        eprintln!(
            "Profile '{}' was created with {} engine but you specified --engine {}.",
            args.profile, profile_engine, cli_engine
        );
        eprint!("Switch to {}? [y/N] ", profile_engine);
        std::io::Write::flush(&mut std::io::stderr())?;
        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;
        if input.trim().eq_ignore_ascii_case("y") {
            // Re-set the engine in the Python bridge to match the profile
            inference::set_engine(profile_engine)
                .map_err(|e| anyhow::anyhow!(e).context("Failed to switch engine"))?;
        } else {
            anyhow::bail!("Engine mismatch. Use --engine {} or choose a different profile.", profile_engine);
        }
    }

    // 2c. Set ChatterBox variant before inference if applicable
    if global.engine == Engine::Chatterbox || profile.profile.engine == "chatterbox" {
        // Use CLI flag if provided, otherwise fall back to profile's stored variant, otherwise default
        let variant_str = args
            .cb_variant
            .map(|v| v.as_str().to_string())
            .or_else(|| profile.profile.cb_variant.clone())
            .unwrap_or_else(|| "original".to_string());
        inference::set_variant(&variant_str)
            .map_err(|e| anyhow::anyhow!(e).context("Failed to set ChatterBox variant"))?;
    }

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

    // ref_text is the transcript of the reference audio (needed for MLX voice cloning)
    let ref_text = &profile.audio.sample_text;

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

    // 6. Preprocess text for natural pacing (always on) and chunk
    let processed = chunk::add_pause_markers(&text);
    let chunks = chunk::chunk_by_paragraph(&processed);
    if chunks.is_empty() {
        anyhow::bail!("No synthesizable text content found");
    }

    // Validate speed multiplier
    let speed = args.speed;
    if speed < 0.5 || speed > 3.0 {
        anyhow::bail!("--speed must be between 0.5 and 3.0 (got {speed})");
    }

    // 7. Load model with spinner (all Python output is suppressed)
    {
        let spinner = ui::create_spinner("Loading model...");
        inference::ensure_model_loaded("custom", quant_override)
            .map_err(|e| anyhow::anyhow!(e))
            .context("Failed to load speech model")?;
        ui::finish_spinner(&spinner, "Model loaded");
    }

    // 8. Synthesize audio with spinner/progress bar
    let mut audio_parts: Vec<(Vec<f32>, u32)> = Vec::with_capacity(chunks.len());
    if chunks.len() == 1 {
        let spinner = ui::create_spinner("Generating audio...");
        let exaggeration = args.exaggeration.unwrap_or(0.5);
        let cfg_weight = args.cfg.unwrap_or(0.5);
        let (wav, sr) = inference::generate_speech(&chunks[0], language_str, &profile_dir, ref_text, false, quant_override, exaggeration, cfg_weight)
            .map_err(|e| anyhow::anyhow!(e))
            .context("Speech generation failed")?;
        audio_parts.push((wav, sr));
        ui::finish_spinner(&spinner, "Audio generated");
    } else {
        let pb = ui::create_progress_bar(chunks.len() as u64, "Generating audio");
        let exaggeration = args.exaggeration.unwrap_or(0.5);
        let cfg_weight = args.cfg.unwrap_or(0.5);
        for chunk_text in &chunks {
            let (wav, sr) = inference::generate_speech(chunk_text, language_str, &profile_dir, ref_text, false, quant_override, exaggeration, cfg_weight)
                .map_err(|e| anyhow::anyhow!(e))
                .context("Speech generation failed")?;
            audio_parts.push((wav, sr));
            pb.inc(1);
        }
        ui::finish_spinner(&pb, "Audio generated");
    }

    // 8b. Apply speed multiplier via WSOLA time-stretching (preserves pitch)
    if (speed - 1.0).abs() > 0.01 {
        let spinner = ui::create_spinner("Adjusting speed...");
        audio_parts = audio_parts
            .into_iter()
            .map(|(wav, sr)| {
                let stretched = audio::time_stretch(&wav, speed);
                (stretched, sr)
            })
            .collect();
        ui::finish_spinner(&spinner, &format!("Speed adjusted ({speed}x)"));
    }

    // 9. Encode to MP3
    let num_parts = audio_parts.len();
    let is_split = args.split && num_parts > 1;
    if is_split {
        for (i, (wav, sr)) in audio_parts.iter().enumerate() {
            let pcm = audio::samples_f32_to_i16(wav);
            let chunk_path = split_output_path(&output_path, i + 1);
            audio::encode_wav_to_mp3(&pcm, *sr, &chunk_path)
                .context("MP3 encoding failed")?;
        }
    } else {
        let (combined, sr) = if audio_parts.len() == 1 {
            audio_parts.into_iter().next().unwrap()
        } else {
            concatenate_chunks(&audio_parts, 300)
        };
        let pcm = audio::samples_f32_to_i16(&combined);
        audio::encode_wav_to_mp3(&pcm, sr, &output_path)
            .context("MP3 encoding failed")?;
    }

    // 10. Unload models to free memory
    let _ = inference::unload_all_models();

    // 11. Print completion
    let file_size = fs::metadata(&output_path)
        .map(|m| format_file_size(m.len()))
        .unwrap_or_default();

    // Resolve to absolute path for the "Done" line
    let abs_output_dir = if output_path.is_absolute() {
        output_path.parent().unwrap_or_else(|| Path::new("/")).to_path_buf()
    } else {
        let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        match output_path.parent() {
            Some(p) if !p.as_os_str().is_empty() => cwd.join(p),
            _ => cwd,
        }
    };

    eprintln!();
    let done = "\u{2714} Done."
        .if_supports_color(Stream::Stderr, |t| t.green().bold().to_string())
        .to_string();
    eprintln!("{done}");

    let dir_str = abs_output_dir.display().to_string();
    let bold_path = dir_str
        .as_str()
        .if_supports_color(Stream::Stderr, |t| t.bold().to_string())
        .to_string();
    eprintln!("\nSaved to: {bold_path}");

    if is_split {
        let stem = output_path.file_stem().unwrap_or_default().to_string_lossy();
        eprintln!(
            "\n\u{1F4C1} Generated {num_parts} files: {stem}-001.mp3 \u{2192} {stem}-{num_parts:03}.mp3",
        );
    } else {
        eprintln!(
            "\n\u{1F3B5} {}  {}",
            output_path.display(),
            format!("({file_size})")
                .if_supports_color(Stream::Stderr, |t| t.dimmed().to_string()),
        );
    }

    // 12. Play audio (skip in split mode)
    if !args.no_play && !is_split {
        eprintln!("\n\u{1F50A} Playing... (press Enter to skip)");
        audio::playback::play_audio_skippable(&output_path).context("Audio playback failed")?;
    }

    Ok(())
}
