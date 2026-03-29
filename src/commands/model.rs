use std::time::Duration;

use anyhow::Context;
use indicatif::{ProgressBar, ProgressStyle};
use owo_colors::OwoColorize;
use owo_colors::Stream::Stderr;

use crate::bridge;
use crate::bridge::model::{size_label, ModelQuantization};
use crate::cli::{GlobalArgs, ModelCommands};

/// Create a spinner with the standard chatter style.
///
/// Shows an animated spinner with the message and elapsed time,
/// matching the UX spec (D-07, D-08).
fn create_spinner(message: &str) -> ProgressBar {
    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::with_template("{spinner:.cyan} {msg} ({elapsed})")
            .expect("valid template")
            .tick_strings(&[
                "\u{28cb}", "\u{28d9}", "\u{28f9}", "\u{28f8}", "\u{28fc}",
                "\u{28f4}", "\u{28e6}", "\u{28e7}", "\u{28c7}", "\u{28cf}",
            ]),
    );
    pb.set_message(message.to_string());
    pb.enable_steady_tick(Duration::from_millis(100));
    pb
}

/// Print an error message to stderr with a red bold "Error:" prefix.
///
/// In verbose mode, prints the full error chain via anyhow's Debug format.
/// In normal mode, prints only the top-level error message.
fn print_error(err: &anyhow::Error, verbose: bool) {
    if verbose {
        eprintln!(
            "{} {err:?}",
            "Error:".if_supports_color(Stderr, |t| t.red())
        );
    } else {
        eprintln!(
            "{} {err}",
            "Error:".if_supports_color(Stderr, |t| t.red())
        );
    }
}

pub fn run(command: ModelCommands, global: &GlobalArgs) -> anyhow::Result<()> {
    match command {
        ModelCommands::Download { variant } => {
            let label = size_label();
            let quant = ModelQuantization::from(variant);
            let suffix = quant.mlx_suffix();

            println!("Downloading Qwen3-TTS {label} ({suffix}) models (VoiceDesign, CustomVoice, Base)...");

            let spinner = create_spinner(&format!("Downloading Qwen3-TTS {label} ({suffix})"));

            match bridge::model::download_model(&quant) {
                Ok(()) => {
                    spinner.finish_with_message(format!("Qwen3-TTS {label} download complete"));
                }
                Err(e) => {
                    spinner.abandon_with_message(format!("Qwen3-TTS {label} download failed"));
                    let err = anyhow::Error::from(e).context("Model download failed");
                    print_error(&err, global.verbose);
                    return Err(err);
                }
            }
        }

        ModelCommands::List => {
            let models = bridge::model::list_cached_models()
                .context("Failed to scan model cache")?;

            if models.is_empty() {
                println!("No models downloaded. Run `chatter model download` to get started.");
            } else {
                println!(
                    "{:<50} {:>10} {}",
                    "Model".if_supports_color(owo_colors::Stream::Stdout, |t| t.bold()),
                    "Size".if_supports_color(owo_colors::Stream::Stdout, |t| t.bold()),
                    "Path".if_supports_color(owo_colors::Stream::Stdout, |t| t.bold()),
                );
                println!("{}", "-".repeat(80));

                for model in &models {
                    let size_str = match model.size_bytes {
                        Some(bytes) => format_bytes(bytes),
                        None => "-".to_string(),
                    };
                    let path_str = model.local_path.as_deref().unwrap_or("-");
                    println!("{:<50} {:>10} {}", model.repo_id, size_str, path_str);
                }
            }
        }

        ModelCommands::Remove => {
            let label = size_label();
            let spinner = create_spinner(&format!("Removing Qwen3-TTS {label}"));

            match bridge::model::remove_model() {
                Ok(()) => {
                    spinner.finish_with_message(format!("Qwen3-TTS {label} removed"));
                }
                Err(bridge::BridgeError::ModelNotFound(msg)) => {
                    spinner.abandon_with_message(msg.clone());
                    println!("{msg}");
                }
                Err(e) => {
                    spinner.abandon_with_message(format!("Failed to remove Qwen3-TTS {label}"));
                    let err = anyhow::Error::from(e).context("Model removal failed");
                    print_error(&err, global.verbose);
                    return Err(err);
                }
            }
        }
    }

    Ok(())
}

/// Format a byte count as a human-readable string (e.g., "3.4 GB").
fn format_bytes(bytes: u64) -> String {
    const GB: f64 = 1_073_741_824.0;
    const MB: f64 = 1_048_576.0;
    const KB: f64 = 1_024.0;

    let b = bytes as f64;
    if b >= GB {
        format!("{:.1} GB", b / GB)
    } else if b >= MB {
        format!("{:.1} MB", b / MB)
    } else if b >= KB {
        format!("{:.1} KB", b / KB)
    } else {
        format!("{bytes} B")
    }
}
