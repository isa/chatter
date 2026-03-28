# Phase 2: Voice Profiles and Speech Generation - Research

**Researched:** 2026-03-27
**Domain:** Voice profile management, TTS inference via PyO3, audio encoding
**Confidence:** MEDIUM (MLX/qwen-tts API split is the primary uncertainty)

## Summary

Phase 2 transforms chatter from a foundation-only CLI into a functional TTS tool. The core work spans four domains: (1) voice profile creation via VoiceDesign and voice cloning, (2) profile storage and management, (3) speech generation from inline text using saved profiles, and (4) audio encoding from WAV to MP3. All Python inference flows through PyO3, building on Phase 1's bridge layer.

The most significant architectural finding is that **MLX models (mlx-community variants) require the `mlx-audio` Python package, NOT `qwen-tts`**. The two packages have different APIs. On CUDA/MPS, `qwen-tts` (Qwen3TTSModel) is the correct package. This means the Python bridge layer needs a backend-aware abstraction that calls the right package depending on the detected compute backend.

**Primary recommendation:** Build a Python-side adapter module (`chatter_bridge.py`) installed into the managed venv that normalizes the `qwen-tts` and `mlx-audio` APIs into a single interface. The Rust PyO3 code calls this adapter, never the underlying packages directly. This keeps the Rust side clean and confines the API differences to one Python file.

<user_constraints>

## User Constraints (from CONTEXT.md)

### Locked Decisions
- **D-01:** Drop `--model-size` flag entirely. Hardcode 1.7B for all operations. Remove 0.6B variant handling from the codebase.
- **D-02:** When MLX backend is detected (Apple Silicon), use `mlx-community/Qwen3-TTS-12Hz-1.7B-*-bf16` model variants instead of `Qwen/Qwen3-TTS-12Hz-1.7B-*`.
- **D-03:** CUDA backend continues using `Qwen/Qwen3-TTS-12Hz-1.7B-*` (original PyTorch variants).
- **D-04:** TOML for profile metadata. Add `toml` crate to Cargo.toml.
- **D-05:** One directory per profile: `~/.config/chatter/profiles/{name}/` containing `profile.toml` and `sample.mp3`.
- **D-06:** Profile metadata includes: name, type (designed/cloned), language, description or source audio path, creation date, model variant used.
- **D-07:** Voice identity stored as raw codes/embeddings from the VoiceDesign model output. Saved to the profile directory for reload at generation time without re-running VoiceDesign.
- **D-08:** When `--name` is omitted, auto-generate from the description by slugifying the first few words. For clone, slugify the source filename.
- **D-09:** After generating the voice, automatically play the cached sample audio using `afplay` on macOS, `aplay`/`paplay` on Linux.
- **D-10:** Interactive design loop: generate -> preview -> accept or retry with modified description.
- **D-11:** Design uses VoiceDesign 1.7B model only.
- **D-12:** No interactive preview loop for clone -- deterministic. Save and print sample path.
- **D-13:** Strict input validation for clone: check file exists, validate format (MP3/WAV), warn if duration too short/long, check sample rate.
- **D-14:** Fixed preview sentence: "Hello, this is a preview of your voice profile."
- **D-15:** Default output path: `./warm-friendly-male-20260327-143022.mp3` (profile name + timestamp).
- **D-16:** Overwrite existing output file with warning.
- **D-17:** Progress bar with percentage during synthesis if qwen-tts provides chunk-level callbacks. Fallback to spinner + elapsed time.
- **D-18:** Add `--play` flag to `generate` command for optional playback.
- **D-19:** `profiles list` shows table: Name, Type, Language, Created. Human-readable only.
- **D-20:** `profiles show {name}` shows full detail dump.

### Claude's Discretion
- Exact TOML schema field names and structure
- How voice embeddings/codes are serialized to disk (binary vs base64 in TOML vs separate file)
- Specific slugification algorithm and collision handling
- Audio validation thresholds for clone (min/max duration, accepted sample rates)
- Progress bar vs spinner decision based on what qwen-tts API actually exposes
- Table formatting library choice or manual formatting for `profiles list`

### Deferred Ideas (OUT OF SCOPE)
None -- discussion stayed within phase scope.

</user_constraints>

<phase_requirements>

## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| PROF-01 | Create voice profile from natural language description via `chatter design` | VoiceDesign API documented; two-step workflow (design -> clone prompt) for reusable profiles |
| PROF-02 | Create voice profile from reference MP3 via `chatter clone` | Base model's `create_voice_clone_prompt` / `generate` with ref_audio documented |
| PROF-03 | Profiles saved to `~/.config/chatter/profiles/` with TOML metadata and cached MP3 | TOML crate, directories crate, profile directory structure documented |
| PROF-04 | List all saved profiles via `chatter profiles list` | Pure Rust file enumeration + TOML parsing; table formatting patterns documented |
| PROF-05 | Profile metadata: name, type, language, description/source, creation date | TOML schema design documented |
| PROF-06 | Cached sample audio generated at profile creation time | Preview sentence pattern + audio encode pipeline documented |
| GEN-01 | Generate speech from inline text using saved profile | Base model `generate_voice_clone` with saved prompt; MLX `generate` with ref_audio |
| GEN-05 | Generated audio saved as MP3 to user-specified or default path | WAV-to-MP3 pipeline with hound + mp3lame-encoder documented |
| GEN-06 | Language flag on generate overrides profile default | Both APIs accept `language` parameter |
| UX-02 | Progress bar during speech synthesis | No granular callbacks found; spinner + elapsed time is the fallback |
| UX-03 | Progress bar during voice profile creation | Same spinner pattern as synthesis |

</phase_requirements>

## Standard Stack

### Core (already in Cargo.toml)
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| pyo3 | 0.28.2 | Python bridge | Already established in Phase 1 |
| clap | 4.5+ | CLI parsing | Already established |
| indicatif | 0.18.4 | Progress/spinners | Already established |
| serde | 1.x | Serialization | Already established |
| directories | 6.0.0 | XDG paths | Already established for venv; reuse for profiles |

### New for Phase 2
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| toml | 0.8+ | Profile metadata serialization | D-04 locked decision. Human-editable format. |
| hound | 3.5.1 | WAV reading | Standard Rust WAV library. Reads PCM output from Python inference before MP3 encoding. |
| mp3lame-encoder | 0.2.2 | WAV-to-MP3 encoding | Statically links LAME. No runtime dependency. Safe Rust bindings. |
| chrono | 0.4+ | Timestamps for profiles and output filenames | D-06 needs creation date, D-15 needs timestamp in filename. |

### Python Packages (managed venv)
| Package | Purpose | Backend |
|---------|---------|---------|
| qwen-tts | TTS inference (CUDA/MPS path) | CUDA, MPS |
| mlx-audio | TTS inference (MLX path) | MLX (Apple Silicon) |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| TOML (toml crate) | serde_json (already a dep) | D-04 locks TOML. JSON would work but user wants human-editable profiles. |
| chrono | time crate | Either works. chrono is more widely used for formatting. |
| Manual table formatting | comfy-table crate | Manual is simpler for a single 4-column table. comfy-table adds a dependency for one use. |

**Installation:**
```bash
# Cargo.toml additions
cargo add toml hound mp3lame-encoder chrono --features chrono/serde
```

## Architecture Patterns

### Project Structure (new modules)
```
src/
  bridge/
    mod.rs           # re-exports
    runtime.rs       # ComputeBackend detection (existing)
    model.rs         # model variant resolution (needs refactoring)
    error.rs         # BridgeError enum (existing, needs new variants)
    inference.rs     # NEW: voice design, clone, generate Python calls
    venv.rs          # venv management (needs mlx-audio install path)
  commands/
    design.rs        # full implementation
    clone.rs         # full implementation
    generate.rs      # full implementation
    profiles.rs      # full implementation
  profile/
    mod.rs           # profile types, storage, loading
    storage.rs       # filesystem operations
  audio/
    mod.rs           # WAV-to-MP3 pipeline
    playback.rs      # afplay/aplay shell-out
  cli.rs             # remove ModelSize, add --play flag
  ui.rs              # existing spinner + new helpers
  main.rs            # existing dispatch
```

### Pattern 1: Backend-Aware Python Bridge

**What:** The Python inference layer must handle two different Python packages (`qwen-tts` for CUDA/MPS, `mlx-audio` for MLX) with different APIs.

**When to use:** Every inference call (design, clone, generate).

**Recommended approach:** Create a Python adapter module that lives in the managed venv:

```python
# chatter_bridge.py -- installed into the managed venv
# Normalizes qwen-tts and mlx-audio into a single interface

import sys
import os

def detect_backend():
    """Return 'mlx', 'cuda', 'mps', or 'cpu'."""
    try:
        import mlx.core as mx
        if mx.metal.is_available():
            return "mlx"
    except ImportError:
        pass
    import torch
    if torch.cuda.is_available():
        return "cuda"
    if hasattr(torch.backends, "mps") and torch.backends.mps.is_available():
        return "mps"
    return "cpu"

def load_design_model(backend):
    if backend == "mlx":
        from mlx_audio.tts.utils import load_model
        return load_model("mlx-community/Qwen3-TTS-12Hz-1.7B-VoiceDesign-bf16")
    else:
        from qwen_tts import Qwen3TTSModel
        import torch
        device = "cuda:0" if backend == "cuda" else "mps"
        return Qwen3TTSModel.from_pretrained(
            "Qwen/Qwen3-TTS-12Hz-1.7B-VoiceDesign",
            device_map=device,
            dtype=torch.bfloat16 if backend == "cuda" else torch.float32,
        )

def voice_design(model, backend, text, language, instruct):
    """Returns (wav_samples, sample_rate) as numpy arrays."""
    if backend == "mlx":
        import numpy as np
        results = list(model.generate_voice_design(
            text=text, language=language, instruct=instruct
        ))
        audio = np.array(results[0].audio)
        return audio, 24000  # Qwen3-TTS outputs 24kHz
    else:
        wavs, sr = model.generate_voice_design(
            text=text, language=language, instruct=instruct
        )
        return wavs[0], sr
```

**Why:** Keeps Rust code clean. PyO3 calls `chatter_bridge.voice_design(model, backend, ...)` -- one interface regardless of backend.

### Pattern 2: Voice Profile Storage

**What:** Each profile is a directory with metadata + cached audio + voice prompt data.

```
~/.config/chatter/profiles/
  warm-friendly-male/
    profile.toml       # metadata
    sample.mp3         # cached preview audio
    voice_prompt.bin   # serialized voice_clone_prompt (torch.save or safetensors)
```

**TOML schema:**
```toml
[profile]
name = "warm-friendly-male"
type = "designed"  # or "cloned"
language = "English"
description = "A warm, friendly male voice with a slight baritone"
# source_audio = "path/to/original.mp3"  # only for cloned profiles
created = "2026-03-27T14:30:22Z"
model_variant = "mlx-community/Qwen3-TTS-12Hz-1.7B-VoiceDesign-bf16"

[audio]
sample_text = "Hello, this is a preview of your voice profile."
sample_rate = 24000
```

**Voice prompt serialization (D-07):** Use `torch.save()` to serialize the `voice_clone_prompt` object to `voice_prompt.bin`. This is a PyTorch tensor bundle. On reload, `torch.load()` restores it and passes directly to `generate_voice_clone(voice_clone_prompt=...)`. For MLX, save the reference audio WAV file instead (`ref_audio.wav`) and re-extract on first use.

### Pattern 3: VoiceDesign -> Reusable Profile (Two-Step Workflow)

**What:** VoiceDesign outputs audio, not reusable embeddings. To make a reusable profile, you must feed the designed audio back through `create_voice_clone_prompt` on the Base model.

**Workflow:**
1. Load VoiceDesign model
2. Call `generate_voice_design(text=preview_sentence, language=lang, instruct=description)` -- produces WAV audio
3. Save the WAV as reference audio
4. Load Base model
5. Call `create_voice_clone_prompt(ref_audio=(wav, sr), ref_text=preview_sentence)` -- produces reusable prompt
6. Serialize prompt to `voice_prompt.bin` via `torch.save()`
7. Encode WAV to MP3 for `sample.mp3`

**This is the canonical pattern from Qwen's own documentation.** The VoiceDesign model is fundamentally a "reference audio generator" -- it creates a voice sample that matches your description, which you then treat as clone source.

**MLX path difference:** mlx-audio does NOT have `create_voice_clone_prompt`. Instead, save the reference audio WAV file to the profile directory and pass it as `ref_audio` on each generation call. This is slightly slower (re-extracts features each time) but functionally equivalent.

### Pattern 4: Audio Pipeline (WAV -> MP3)

**What:** Python inference returns raw PCM audio (numpy array + sample rate). Rust handles encoding to MP3.

**Flow:**
1. Python returns `(numpy_array, sample_rate)` via PyO3
2. Rust extracts the numpy array as `Vec<f32>` or `Vec<i16>`
3. Convert to PCM format expected by mp3lame-encoder
4. Encode to MP3 with mp3lame-encoder
5. Write to file

```rust
use hound::{WavSpec, WavWriter};
use mp3lame_encoder::{Builder, Encoder, FlushNoGap, MonoPcm};

fn wav_to_mp3(samples: &[i16], sample_rate: u32, output_path: &Path) -> anyhow::Result<()> {
    let mut encoder = Builder::new()
        .expect("valid builder")
        .set_num_channels(1)
        .expect("valid channels")
        .set_sample_rate(sample_rate)
        .expect("valid sample rate")
        .set_brate(mp3lame_encoder::Bitrate::Kbps192)
        .expect("valid bitrate")
        .set_quality(mp3lame_encoder::Quality::Best)
        .expect("valid quality")
        .build()
        .expect("valid encoder");

    let input = MonoPcm(samples);
    let mut output = vec![0u8; mp3lame_encoder::max_required_buffer_size(samples.len())];
    let encoded_size = encoder.encode(&input, &mut output)?;
    let flush_size = encoder.flush::<FlushNoGap>(&mut output[encoded_size..])?;
    output.truncate(encoded_size + flush_size);

    std::fs::write(output_path, &output)?;
    Ok(())
}
```

### Anti-Patterns to Avoid
- **Calling Python packages directly from Rust without an adapter:** The qwen-tts and mlx-audio APIs differ significantly. Without an adapter, every Rust function would need backend-conditional logic.
- **Storing voice profiles as a single flat JSON/TOML file:** D-05 locks the one-directory-per-profile structure. A single file doesn't scale and can't hold binary data (audio, tensors).
- **Re-running VoiceDesign on every generate call:** VoiceDesign is expensive (~10-30 seconds). Always save and reuse the clone prompt or reference audio.
- **Using async for inference:** D-01 from Phase 1 -- model inference is GPU-bound and blocking. No async needed.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| WAV reading | Custom PCM parser | `hound` crate | WAV format has many variants (8/16/24/32 bit, int/float, compression). hound handles all of them. |
| MP3 encoding | FFmpeg subprocess | `mp3lame-encoder` | Static linking, no runtime dependency, no subprocess failure modes. |
| XDG directories | `~/.config` hardcoding | `directories` crate | Cross-platform. Already used for venv path in Phase 1. |
| Slug generation | Manual string manipulation | Simple function with regex | Only need lowercase + hyphen replacement. ~10 lines, not worth a crate. |
| TOML serialization | Manual string building | `toml` crate with serde derive | Handles escaping, types, nested tables correctly. |
| Audio playback | Audio crate (rodio, cpal) | `afplay` / `aplay` shell-out (D-09) | User decision. System commands are simpler and already available. |

## Common Pitfalls

### Pitfall 1: MLX vs qwen-tts API Mismatch
**What goes wrong:** Code written for `qwen-tts` (PyTorch) fails on MLX because `mlx-audio` has a different API (e.g., returns `mx.array` not numpy, uses `generate()` not `generate_voice_clone()`, returns generator objects not tuples).
**Why it happens:** D-02 requires MLX variants on Apple Silicon, but the two packages have fundamentally different interfaces.
**How to avoid:** Use a Python adapter module that normalizes both APIs. Test both paths.
**Warning signs:** Code that directly calls `Qwen3TTSModel.from_pretrained()` from Rust without checking backend.

### Pitfall 2: Voice Prompt Serialization Across Backends
**What goes wrong:** A voice prompt saved with `torch.save()` on CUDA cannot be loaded on MLX, and vice versa.
**Why it happens:** PyTorch tensors and MLX arrays are incompatible formats.
**How to avoid:** For CUDA/MPS path, use `torch.save()` for the voice_clone_prompt. For MLX path, save the reference WAV audio file instead and re-extract features at generation time. The profile.toml `type` field distinguishes which approach was used.
**Warning signs:** Profile created on one machine doesn't work on another with different backend.

### Pitfall 3: PyO3 Numpy Array Extraction
**What goes wrong:** Extracting numpy arrays from Python to Rust via PyO3 can fail silently or produce wrong data if types don't match.
**Why it happens:** Python returns float32 numpy arrays; Rust expects specific types. The numpy array might be on GPU (CUDA/MPS) and needs `.cpu().numpy()` first.
**How to avoid:** Always call `.cpu().numpy()` in the Python adapter before returning to Rust. Use `PyArrayDyn<f32>` from `numpy` PyO3 bindings or extract as a Python list.
**Warning signs:** Silent zeros in audio output, or PyO3 type extraction errors.

### Pitfall 4: MP3 Encoding Sample Format
**What goes wrong:** mp3lame-encoder expects integer PCM samples, but model output is float32.
**Why it happens:** Neural TTS models produce normalized float audio in [-1.0, 1.0] range.
**How to avoid:** Convert float32 to i16 before encoding: `(sample * 32767.0).clamp(-32768.0, 32767.0) as i16`.
**Warning signs:** Extremely loud static/noise in output MP3.

### Pitfall 5: Profile Name Collision
**What goes wrong:** Two profiles with auto-generated names collide (e.g., "warm-friendly-male" already exists).
**Why it happens:** D-08 auto-generates names from description slugification.
**How to avoid:** Check if directory exists, append `-2`, `-3`, etc. until unique.
**Warning signs:** Overwriting an existing profile without warning.

### Pitfall 6: VoiceDesign Model Loading Time
**What goes wrong:** The interactive design loop (D-10) feels sluggish because each retry reloads the model.
**Why it happens:** Model loading takes 10-30 seconds. If the model is dropped between retries, each attempt pays the full cost.
**How to avoid:** Keep the model loaded in memory during the entire design session. Only drop it when the user accepts or exits.
**Warning signs:** Long waits between retries in the design loop.

### Pitfall 7: Venv Package Installation for Correct Backend
**What goes wrong:** The venv installs `qwen-tts` but user is on Apple Silicon and needs `mlx-audio`.
**Why it happens:** Phase 1 venv setup hardcodes `qwen-tts` as the only required package.
**How to avoid:** Detect backend BEFORE venv setup. Install `mlx-audio` on MLX systems, `qwen-tts` on CUDA/MPS. Or install both (heavier but simpler).
**Warning signs:** `ModuleNotFoundError: No module named 'mlx_audio'` on Mac.

## Code Examples

### PyO3: Calling Python Inference and Extracting Audio

```rust
// Source: PyO3 user guide + project patterns
use pyo3::prelude::*;

fn run_voice_design(
    description: &str,
    language: &str,
    text: &str,
) -> Result<(Vec<f32>, u32), BridgeError> {
    Python::attach(|py| {
        let bridge = py.import("chatter_bridge")?;
        let result = bridge.call_method1(
            "voice_design_and_extract",
            (description, language, text),
        )?;
        // Returns (list_of_floats, sample_rate)
        let wav: Vec<f32> = result.get_item(0)?.extract()?;
        let sr: u32 = result.get_item(1)?.extract()?;
        Ok((wav, sr))
    })
}
```

### Profile TOML Read/Write with Serde

```rust
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
struct ProfileMetadata {
    profile: ProfileInfo,
    audio: AudioInfo,
}

#[derive(Serialize, Deserialize)]
struct ProfileInfo {
    name: String,
    #[serde(rename = "type")]
    profile_type: String,  // "designed" or "cloned"
    language: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    source_audio: Option<String>,
    created: String,  // ISO 8601
    model_variant: String,
}

#[derive(Serialize, Deserialize)]
struct AudioInfo {
    sample_text: String,
    sample_rate: u32,
}

fn save_profile(profile: &ProfileMetadata, dir: &Path) -> anyhow::Result<()> {
    std::fs::create_dir_all(dir)?;
    let toml_str = toml::to_string_pretty(profile)?;
    std::fs::write(dir.join("profile.toml"), toml_str)?;
    Ok(())
}

fn load_profile(dir: &Path) -> anyhow::Result<ProfileMetadata> {
    let content = std::fs::read_to_string(dir.join("profile.toml"))?;
    let profile: ProfileMetadata = toml::from_str(&content)?;
    Ok(profile)
}
```

### Slugification

```rust
fn slugify(input: &str, max_words: usize) -> String {
    input
        .to_lowercase()
        .split_whitespace()
        .take(max_words)
        .collect::<Vec<_>>()
        .join("-")
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '-')
        .collect::<String>()
        .trim_matches('-')
        .to_string()
}

fn unique_profile_name(base: &str, profiles_dir: &Path) -> String {
    if !profiles_dir.join(base).exists() {
        return base.to_string();
    }
    for i in 2.. {
        let candidate = format!("{base}-{i}");
        if !profiles_dir.join(&candidate).exists() {
            return candidate;
        }
    }
    unreachable!()
}
```

### Audio Playback Shell-Out (D-09)

```rust
use std::process::Command;

fn play_audio(path: &Path) -> anyhow::Result<()> {
    let cmd = if cfg!(target_os = "macos") {
        "afplay"
    } else {
        // Try paplay first (PulseAudio), then aplay (ALSA)
        if Command::new("paplay").arg("--version").output().is_ok() {
            "paplay"
        } else {
            "aplay"
        }
    };

    let status = Command::new(cmd)
        .arg(path)
        .status()
        .context("Failed to play audio")?;

    if !status.success() {
        anyhow::bail!("Audio playback failed with status {status}");
    }
    Ok(())
}
```

### Interactive Design Loop (D-10)

```rust
use std::io::{self, Write};

fn design_loop(initial_desc: &str, language: &str, global: &GlobalArgs) -> anyhow::Result<ProfileMetadata> {
    let mut description = initial_desc.to_string();
    let preview_text = "Hello, this is a preview of your voice profile.";

    // Load model once, keep for entire session
    let spinner = ui::create_spinner("Loading VoiceDesign model...");
    // ... load model via PyO3 ...
    spinner.finish_and_clear();

    loop {
        let spinner = ui::create_spinner("Designing voice...");
        let (wav, sr) = run_voice_design(&description, language, preview_text)?;
        spinner.finish_and_clear();

        // Encode to MP3 and save temp file
        let temp_mp3 = encode_to_temp_mp3(&wav, sr)?;

        // Play preview (D-09)
        println!("Preview of your custom voice...");
        play_audio(&temp_mp3)?;

        // Ask user
        print!("Accept this voice? [Y/n/retry with new description] ");
        io::stdout().flush()?;
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;

        match input.trim().to_lowercase().as_str() {
            "" | "y" | "yes" => {
                // Save profile and return
                break;
            }
            "n" | "no" => {
                anyhow::bail!("Voice design cancelled");
            }
            new_desc => {
                description = new_desc.to_string();
            }
        }
    }
    // ... save profile ...
}
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| qwen-tts only | qwen-tts (CUDA/MPS) + mlx-audio (MLX) | Jan 2026 | MLX variants need separate package |
| VoiceDesign as direct embeddings | VoiceDesign -> audio -> clone prompt | Jan 2026 | Two-step workflow required for reusable profiles |
| Flash Attention 2 required | SDPA works on MPS, FA2 on CUDA | Ongoing | Use `attn_implementation="sdpa"` on Mac |

## Open Questions

1. **MLX voice prompt caching**
   - What we know: `mlx-audio` does NOT have `create_voice_clone_prompt`. You pass `ref_audio` each time.
   - What's unclear: Whether `mlx-audio` caches internally, or if every generation re-extracts features from the ref audio.
   - Recommendation: Save reference audio WAV in profile directory. Accept the potential per-generation overhead on MLX. Benchmark actual impact -- it may be negligible for single sentences.

2. **Numpy array transfer efficiency via PyO3**
   - What we know: PyO3 can extract Python lists. The `numpy` PyO3 crate exists but adds a dependency.
   - What's unclear: Whether extracting a list of 100k+ floats via PyO3 is fast enough, or if we need the numpy crate for zero-copy.
   - Recommendation: Start with Python list extraction (simpler). If profiling shows it's a bottleneck, add `numpy` crate for zero-copy.

3. **Venv dual-package installation**
   - What we know: `qwen-tts` and `mlx-audio` have different (large) dependency trees. Installing both wastes disk.
   - What's unclear: Whether installing both causes conflicts (both depend on different PyTorch versions potentially).
   - Recommendation: Detect backend first, install only the needed package. Update `venv.rs` to be backend-aware.

4. **D-07 interpretation for MLX backend**
   - What we know: D-07 says "voice identity stored as raw codes/embeddings." On CUDA/MPS, this is the `voice_clone_prompt` object via `torch.save()`. On MLX, there's no equivalent serializable prompt object.
   - What's unclear: Whether the user expects binary tensor files specifically, or is okay with reference audio WAV as the "stored identity" on MLX.
   - Recommendation: On MLX, store the reference audio WAV + metadata as the voice identity. Document this as a platform difference. The end-user experience is identical (profile works, voice sounds the same).

## Project Constraints (from CLAUDE.md)

- **Tech stack:** Rust CLI with PyO3 -- no subprocess calls for inference (subprocess only for audio playback per D-09)
- **Distribution:** `brew install chatter` must work. Managed venv at `~/.local/share/chatter/venv/`
- **No async:** Synchronous only. No tokio/async-std.
- **No audio playback crates:** rodio/cpal explicitly excluded. Shell-out only.
- **No SQLite:** Profiles are individual files, not a database.
- **Error handling:** anyhow for top-level, thiserror for bridge layer.
- **Colors:** owo-colors with `if_supports_color` for NO_COLOR compliance.
- **Progress:** indicatif spinners/progress bars.

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| afplay | Audio preview (D-09) | Yes | macOS built-in | -- |
| Python 3.12+ | PyO3 runtime | Yes (via venv.rs detection) | -- | -- |
| Rust 1.85+ | Build | Yes (Phase 1 complete) | -- | -- |

**Missing dependencies with no fallback:** None.

## Sources

### Primary (HIGH confidence)
- [Qwen3-TTS GitHub](https://github.com/QwenLM/Qwen3-TTS) - VoiceDesign API, clone prompt workflow, model variant IDs
- [qwen-tts PyPI](https://pypi.org/project/qwen-tts/) - Python package API (generate_voice_design, generate_voice_clone, create_voice_clone_prompt)
- [mlx-audio GitHub](https://github.com/Blaizzy/mlx-audio) - MLX-specific API (generate, generate_voice_design, generate_custom_voice)
- [mlx-audio Qwen3-TTS README](https://github.com/Blaizzy/mlx-audio/blob/main/mlx_audio/tts/models/qwen3_tts/README.md) - MLX model loading and generation patterns

### Secondary (MEDIUM confidence)
- [mlx-community HuggingFace](https://huggingface.co/mlx-community) - Verified all three 1.7B bf16 variants exist: VoiceDesign-bf16, CustomVoice-bf16, Base-bf16
- [DeepWiki Prompt Reuse](https://deepwiki.com/l-xiaoshen/Qwen3-TTS/6.2-prompt-reuse-and-caching) - VoiceClonePromptItem data structure (ref_code, ref_spk_embedding, x_vector_only_mode, icl_mode, ref_text)
- [mp3lame-encoder docs.rs](https://docs.rs/mp3lame-encoder/latest/mp3lame_encoder/) - Encoder API, Builder pattern, MonoPcm/DualPcm input types
- [ComfyUI-Qwen-TTS voice saving](https://deepwiki.com/flybirdxx/ComfyUI-Qwen-TTS/5.3-voice-cloning-and-saving) - torch.save pattern for voice prompts

### Tertiary (LOW confidence)
- [Q3-TTS Apple Silicon repo](https://github.com/esendjer/Q3-TTS) - device_map="mps" with dtype=torch.float32 on Mac (unverified against qwen-tts specifically)

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH - all crates are established, versions verified from CLAUDE.md
- Architecture: MEDIUM - the dual-backend (mlx-audio vs qwen-tts) pattern is novel and needs hands-on validation
- Voice design workflow: MEDIUM - two-step pattern documented by Qwen but the MLX path lacks prompt serialization
- Audio pipeline: HIGH - hound + mp3lame-encoder are straightforward
- Pitfalls: HIGH - identified from concrete API differences across backends

**Research date:** 2026-03-27
**Valid until:** 2026-04-27 (stable domain, 30-day validity)
