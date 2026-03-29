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
/// Uses stdin in its default cooked mode — no raw mode, no terminal
/// manipulation, no threads. This is the only approach that doesn't
/// interfere with dialoguer's own terminal handling across multiple calls.
pub fn play_audio_skippable(path: &Path) -> anyhow::Result<()> {
    let cmd = player_cmd();
    let mut child = Command::new(cmd)
        .arg(path)
        .spawn()
        .with_context(|| format!("Failed to start audio player ({cmd})"))?;

    // Poll stdin (fd 0) for Enter key while checking if child is still alive.
    // Stdin stays in cooked/canonical mode — the OS handles line editing.
    // We only detect a completed line (Enter press), which is simpler but
    // completely safe for dialoguer to use afterwards.
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) => {}
            Err(_) => break,
        }

        let mut pollfd = libc::pollfd {
            fd: 0, // stdin
            events: libc::POLLIN,
            revents: 0,
        };
        let ready = unsafe { libc::poll(&mut pollfd, 1, 50) };
        if ready > 0 && (pollfd.revents & libc::POLLIN) != 0 {
            // Consume the line from stdin
            let mut buf = String::new();
            let _ = std::io::stdin().read_line(&mut buf);
            let _ = child.kill();
            let _ = child.wait();
            break;
        }
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
