use std::fs;
use std::path::{Path, PathBuf};

use anyhow::Context;
use owo_colors::{OwoColorize, Stream, Style};

use crate::audio;
use crate::bridge::inference;
use crate::chunk;
use crate::cli::{GenerateArgs, GlobalArgs, Language};
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

    // 7. Chunk the text per D-03
    let chunks = chunk::chunk_by_paragraph(&text);
    if chunks.is_empty() {
        anyhow::bail!("No synthesizable text content found");
    }

    // 8. Multi-chunk synthesis with progress per D-11, D-12
    let mut audio_parts: Vec<(Vec<f32>, u32)> = Vec::with_capacity(chunks.len());
    if chunks.len() == 1 {
        // Single chunk: use spinner (same as inline text behavior)
        let spinner = ui::create_spinner("Generating speech...");
        let (wav, sr) = inference::generate_speech(&chunks[0], language_str, &profile_dir)
            .map_err(|e| anyhow::anyhow!(e))
            .context("Speech generation failed")?;
        audio_parts.push((wav, sr));
        spinner.finish_and_clear();
    } else {
        // Multiple chunks: use bounded progress bar per D-12
        let pb = ui::create_progress_bar(chunks.len() as u64, "Synthesizing");
        for chunk_text in &chunks {
            let (wav, sr) = inference::generate_speech(chunk_text, language_str, &profile_dir)
                .map_err(|e| anyhow::anyhow!(e))
                .context("Speech generation failed")?;
            audio_parts.push((wav, sr));
            pb.inc(1);
        }
        pb.finish_and_clear();
    }

    // 9. Encode to MP3 per D-04
    if args.split && audio_parts.len() > 1 {
        // D-04: Split mode -- separate file per chunk
        let spinner = ui::create_spinner("Encoding MP3 files...");
        for (i, (wav, sr)) in audio_parts.iter().enumerate() {
            let pcm = audio::samples_f32_to_i16(wav);
            let chunk_path = split_output_path(&output_path, i + 1);
            audio::encode_wav_to_mp3(&pcm, *sr, &chunk_path)
                .context("MP3 encoding failed")?;
        }
        spinner.finish_and_clear();
        eprintln!(
            "Generated {} files: {}-001.mp3 through {}-{:03}.mp3",
            audio_parts.len(),
            output_path
                .file_stem()
                .unwrap_or_default()
                .to_string_lossy(),
            output_path
                .file_stem()
                .unwrap_or_default()
                .to_string_lossy(),
            audio_parts.len()
        );
    } else {
        // Default: single concatenated output
        let spinner = ui::create_spinner("Encoding MP3...");
        let (combined, sr) = if audio_parts.len() == 1 {
            audio_parts.into_iter().next().unwrap()
        } else {
            concatenate_chunks(&audio_parts, 300) // 300ms silence gap between chunks
        };
        let pcm = audio::samples_f32_to_i16(&combined);
        audio::encode_wav_to_mp3(&pcm, sr, &output_path)
            .context("MP3 encoding failed")?;
        spinner.finish_and_clear();

        // Print success with file size
        let file_size = fs::metadata(&output_path)
            .map(|m| format_file_size(m.len()))
            .unwrap_or_else(|_| "unknown size".to_string());
        eprintln!("Generated: {} ({})", output_path.display(), file_size);
    }

    // 10. Unload models to free memory
    let _ = inference::unload_all_models();

    // 11. Play audio per D-18 (skip in split mode -- no single file to play)
    if args.play && !args.split {
        audio::playback::play_audio(&output_path).context("Audio playback failed")?;
    }

    Ok(())
}
