use std::path::Path;
use std::process::Command;
use std::sync::atomic::{AtomicU32, Ordering};

use anyhow::Context;

/// Global PID of the currently-playing audio child process.
/// When chatter receives SIGINT/SIGTERM, the signal handler kills this PID
/// so afplay doesn't keep playing after chatter exits.
static PLAYER_PID: AtomicU32 = AtomicU32::new(0);

/// Install a one-time signal handler that kills the audio player on Ctrl+C.
fn install_signal_handler() {
    use std::sync::Once;
    static INIT: Once = Once::new();
    INIT.call_once(|| {
        unsafe {
            libc::signal(libc::SIGINT, signal_handler as *const () as libc::sighandler_t);
            libc::signal(libc::SIGTERM, signal_handler as *const () as libc::sighandler_t);
        }
    });
}

extern "C" fn signal_handler(sig: libc::c_int) {
    let pid = PLAYER_PID.load(Ordering::SeqCst);
    if pid != 0 {
        unsafe { libc::kill(pid as i32, libc::SIGTERM) };
    }
    // Re-raise to let the default handler terminate chatter
    unsafe {
        libc::signal(sig, libc::SIG_DFL);
        libc::raise(sig);
    }
}

/// Play an audio file using the system's default audio player.
/// macOS: afplay, Linux: paplay (PulseAudio) or aplay (ALSA).
///
/// Blocks until playback finishes. Ctrl+C kills both chatter and afplay.
pub fn play_audio(path: &Path) -> anyhow::Result<()> {
    install_signal_handler();
    let cmd = player_cmd();
    let mut child = Command::new(cmd)
        .arg(path)
        .spawn()
        .with_context(|| format!("Failed to play audio with {cmd}"))?;

    PLAYER_PID.store(child.id(), Ordering::SeqCst);
    let status = child.wait()
        .with_context(|| format!("Failed to wait for {cmd}"))?;
    PLAYER_PID.store(0, Ordering::SeqCst);

    if !status.success() && status.code() != Some(9) {
        // code 9 = SIGKILL from our skip, not a real error
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
/// - Ctrl+C (signal handler kills afplay, then chatter exits)
///
/// Uses stdin in cooked mode — no raw mode, no terminal manipulation.
pub fn play_audio_skippable(path: &Path) -> anyhow::Result<()> {
    install_signal_handler();
    let cmd = player_cmd();
    let mut child = Command::new(cmd)
        .arg(path)
        .spawn()
        .with_context(|| format!("Failed to start audio player ({cmd})"))?;

    PLAYER_PID.store(child.id(), Ordering::SeqCst);

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
            let mut buf = String::new();
            let _ = std::io::stdin().read_line(&mut buf);
            let _ = child.kill();
            let _ = child.wait();
            break;
        }
    }

    PLAYER_PID.store(0, Ordering::SeqCst);
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
