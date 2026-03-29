use std::io::BufRead;
use std::path::Path;
use std::process::Command;

use anyhow::Context;

/// Play an audio file using the system's default audio player.
/// macOS: afplay, Linux: paplay (PulseAudio) or aplay (ALSA).
///
/// Blocks until playback finishes.
pub fn play_audio(path: &Path) -> anyhow::Result<()> {
    let cmd = player_cmd();
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

/// Play an audio file in the background, skippable by pressing Enter.
///
/// Spawns the player as a child process and waits for either:
/// - The playback to finish naturally, or
/// - The user to press Enter (kills the player process)
///
/// Returns Ok(()) in both cases.
pub fn play_audio_skippable(path: &Path) -> anyhow::Result<()> {
    let cmd = player_cmd();
    let mut child = Command::new(cmd)
        .arg(path)
        .spawn()
        .with_context(|| format!("Failed to start audio player ({cmd})"))?;

    // Spawn a thread to wait for Enter key
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let stdin = std::io::stdin();
        let mut line = String::new();
        let _ = stdin.lock().read_line(&mut line);
        let _ = tx.send(());
    });

    // Poll: check if child exited or user pressed Enter
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break, // playback finished
            Ok(None) => {}        // still playing
            Err(_) => break,      // error, stop
        }
        if rx.try_recv().is_ok() {
            // User pressed Enter — kill the player
            let _ = child.kill();
            let _ = child.wait();
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }

    Ok(())
}

fn player_cmd() -> &'static str {
    if cfg!(target_os = "macos") {
        "afplay"
    } else if Command::new("paplay")
        .arg("--version")
        .output()
        .is_ok()
    {
        "paplay"
    } else {
        "aplay"
    }
}
