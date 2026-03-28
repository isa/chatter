# Phase 2: Voice Profiles and Speech Generation - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md -- this log preserves the alternatives considered.

**Date:** 2026-03-27
**Phase:** 02-voice-profiles-and-speech-generation
**Areas discussed:** Profile Storage Format, Voice Design Flow, Speech Generation Output, Profile Listing Display

---

## Profile Storage Format

| Option | Description | Selected |
|--------|-------------|----------|
| TOML | Human-editable, idiomatic for Rust config. Users could hand-tweak profiles. | ✓ |
| JSON | Already in Cargo.toml, simpler parsing. Profiles are data, not hand-edited config. | |

**User's choice:** TOML
**Notes:** User values human-editability of profiles.

---

| Option | Description | Selected |
|--------|-------------|----------|
| One directory per profile | ~/.config/chatter/profiles/{name}/ with metadata file + sample.mp3 inside | ✓ |
| Flat with naming convention | ~/.config/chatter/profiles/{name}.toml + {name}.mp3 side by side | |

**User's choice:** One directory per profile
**Notes:** None.

---

| Option | Description | Selected |
|--------|-------------|----------|
| Short fixed sentence | e.g. "Hello, this is a preview of your voice profile" (~3-5 seconds) | ✓ |
| User provides sample text | Optional --sample-text flag, fallback to a fixed sentence | |
| You decide | Claude picks a reasonable approach | |

**User's choice:** Short fixed sentence
**Notes:** None.

---

| Option | Description | Selected |
|--------|-------------|----------|
| Auto-generate from description | Slugify first few words (e.g. "warm friendly male" -> warm-friendly-male) | ✓ |
| Prompt the user | Ask interactively for a name before saving | |
| Require --name | Make it a required flag, fail if missing | |

**User's choice:** Auto-generate from description
**Notes:** None.

---

## Voice Design Flow

| Option | Description | Selected |
|--------|-------------|----------|
| Store raw codes/embeddings | Save model output directly, reload at generation time | ✓ |
| Store description text | Re-run VoiceDesign each time (slower but simpler storage) | |
| You decide | Claude picks based on qwen-tts API | |

**User's choice:** Store raw codes/embeddings
**Notes:** None.

---

| Option | Description | Selected |
|--------|-------------|----------|
| Minimal validation | Just check file exists and is a valid audio file | |
| Strict validation | Validate format (MP3/WAV), duration (warn if too short/long), sample rate | ✓ |
| You decide | Claude picks reasonable validation | |

**User's choice:** Strict validation for clone input audio
**Notes:** None.

---

| Option | Description | Selected |
|--------|-------------|----------|
| Silent save | Just save the profile and confirm with a message | |
| Print the path | Save and print the sample MP3 path | |
| Interactive preview | Auto-play sample in terminal, allow retry with modified description | ✓ |

**User's choice:** Interactive preview with retry loop (free-text response)
**Notes:** User wants automatic playback via system command after voice generation, with option to tweak description and regenerate until satisfied. Shell out to afplay (Mac) / aplay (Linux).

---

| Option | Description | Selected |
|--------|-------------|----------|
| Ignore --model-size | Always use 1.7B for everything | ✓ (expanded) |

**User's choice:** Drop --model-size flag entirely, only support 1.7B
**Notes:** User explicitly wants to remove the flag altogether, not just ignore it for design.

---

| Option | Description | Selected |
|--------|-------------|----------|
| Apply MLX variants to all | Use mlx-community/* for all three model types on MLX | ✓ |
| CustomVoice only | Only inference model needs MLX optimization | |
| You decide | Claude researches available variants | |

**User's choice:** Apply MLX variants to all three model types
**Notes:** User referenced https://huggingface.co/mlx-community/Qwen3-TTS-12Hz-1.7B-CustomVoice-bf16 as example. Researcher must verify all three variants exist.

---

| Option | Description | Selected |
|--------|-------------|----------|
| Both design and clone | Consistent preview-and-retry for both | |
| Design only | Clone is deterministic, just save and show path | ✓ |
| You decide | | |

**User's choice:** Design only gets interactive preview loop
**Notes:** Clone is deterministic -- same input produces same output.

---

## Speech Generation Output

| Option | Description | Selected |
|--------|-------------|----------|
| Auto-name (output.mp3) | Current directory with generic name | |
| Profile+timestamp | e.g. ./warm-friendly-male-20260327-143022.mp3 | ✓ |
| You decide | Claude picks a naming scheme | |

**User's choice:** Profile name + timestamp
**Notes:** None.

---

| Option | Description | Selected |
|--------|-------------|----------|
| Overwrite silently | Replace without notice | |
| Overwrite with warning | Print a note that file was replaced | ✓ |
| Fail with error | Refuse to overwrite | |
| You decide | | |

**User's choice:** Overwrite with warning
**Notes:** None.

---

| Option | Description | Selected |
|--------|-------------|----------|
| Spinner only | Same as model loading style | |
| Progress bar with percentage | If chunk-level progress available | ✓ |
| You decide | Use whatever qwen-tts exposes | |

**User's choice:** Progress bar with percentage, fallback to spinner
**Notes:** User wants real progress bar if available, spinner as fallback.

---

| Option | Description | Selected |
|--------|-------------|----------|
| No playback | Just save and print path | |
| Auto-play | Always play after generation | |
| Optional --play flag | User chooses at invocation time | ✓ |

**User's choice:** Optional --play flag
**Notes:** None.

---

## Profile Listing Display

| Option | Description | Selected |
|--------|-------------|----------|
| Simple table | Name, Type, Language, Created date. One line per profile. | ✓ |
| Compact list | Just names, scriptable | |
| Rich table | Name, Type, Language, Description, Created, Sample path. Colorized. | |
| You decide | | |

**User's choice:** Simple table
**Notes:** None.

---

| Option | Description | Selected |
|--------|-------------|----------|
| Full detail dump | All metadata, sample path, description/source, file sizes | ✓ |
| Metadata + playback | Show all metadata AND auto-play cached sample | |
| You decide | | |

**User's choice:** Full detail dump (no auto-play)
**Notes:** None.

---

| Option | Description | Selected |
|--------|-------------|----------|
| No --json | Human-readable only for v1 | ✓ |
| Yes --json | JSON array output when flag passed | |
| You decide | | |

**User's choice:** No --json for v1
**Notes:** Keep it simple.

---

## Claude's Discretion

- Exact TOML schema field names and structure
- Voice embedding serialization format (binary vs base64 vs separate file)
- Slugification algorithm and collision handling
- Audio validation thresholds for clone (min/max duration, sample rates)
- Progress bar vs spinner decision based on qwen-tts API capabilities
- Table formatting approach for profiles list

## Deferred Ideas

None -- discussion stayed within phase scope.
