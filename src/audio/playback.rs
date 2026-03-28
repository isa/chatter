use std::path::Path;
use std::process::Command;

use anyhow::Context;

/// Play an audio file using the system's default audio player.
/// macOS: afplay, Linux: paplay (PulseAudio) or aplay (ALSA).
pub fn play_audio(path: &Path) -> anyhow::Result<()> {
    let cmd = if cfg!(target_os = "macos") {
        "afplay"
    } else if Command::new("paplay")
        .arg("--version")
        .output()
        .is_ok()
    {
        "paplay"
    } else {
        "aplay"
    };

    let status = Command::new(cmd)
        .arg(path)
        .status()
        .with_context(|| format!("Failed to play audio with {cmd}"))?;

    if !status.success() {
        anyhow::bail!(
            "Audio playback failed (exit code: {})",
            status.code().unwrap_or(-1)
        );
    }
    Ok(())
}
