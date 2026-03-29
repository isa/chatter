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

/// Play an audio file in the background, skippable by pressing any key.
///
/// Spawns the player as a child process and waits for either:
/// - The playback to finish naturally, or
/// - The user to press any key (kills the player process)
///
/// Uses a single-threaded approach: opens /dev/tty directly, puts it in
/// raw mode, and polls for input with a timeout. No orphaned threads that
/// could corrupt terminal state across multiple calls.
pub fn play_audio_skippable(path: &Path) -> anyhow::Result<()> {
    use std::fs::File;
    use std::io::Read;
    use std::os::unix::io::AsRawFd;

    let cmd = player_cmd();
    let mut child = Command::new(cmd)
        .arg(path)
        .spawn()
        .with_context(|| format!("Failed to start audio player ({cmd})"))?;

    // Open /dev/tty directly for key detection
    let tty = File::open("/dev/tty").context("Failed to open /dev/tty")?;
    let fd = tty.as_raw_fd();

    // Save original terminal settings and switch to raw mode
    let original_termios = unsafe {
        let mut termios = std::mem::zeroed::<libc::termios>();
        libc::tcgetattr(fd, &mut termios);
        termios
    };

    let mut raw = original_termios;
    // Disable canonical mode and echo — read keys immediately without waiting for Enter
    raw.c_lflag &= !(libc::ICANON | libc::ECHO);
    raw.c_cc[libc::VMIN] = 0;  // non-blocking
    raw.c_cc[libc::VTIME] = 0; // no timeout
    unsafe { libc::tcsetattr(fd, libc::TCSANOW, &raw) };

    // Poll loop: check child process and tty for input
    let mut buf = [0u8; 8];
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) => {}
            Err(_) => break,
        }

        // Poll /dev/tty with a 50ms timeout
        let mut pollfd = libc::pollfd {
            fd,
            events: libc::POLLIN,
            revents: 0,
        };
        let ready = unsafe { libc::poll(&mut pollfd, 1, 50) };
        if ready > 0 && (pollfd.revents & libc::POLLIN) != 0 {
            // Drain the input buffer
            let tty_ref = &tty;
            let _ = (&*tty_ref).read(&mut buf);
            // Kill the player
            let _ = child.kill();
            let _ = child.wait();
            break;
        }
    }

    // Restore original terminal settings — critical for dialoguer to work
    unsafe { libc::tcsetattr(fd, libc::TCSANOW, &original_termios) };

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
