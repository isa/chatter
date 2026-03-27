# Feature Research

**Domain:** TTS CLI tools (local inference, voice profile management)
**Researched:** 2026-03-27
**Confidence:** MEDIUM-HIGH

## Feature Landscape

### Table Stakes (Users Expect These)

Features users assume exist. Missing these = product feels incomplete.

| Feature | Why Expected | Complexity | Notes |
|---------|--------------|------------|-------|
| Text-to-speech from string input | Core purpose of any TTS tool; every competitor supports `--text "hello"` | LOW | Piper, edge-tts, coqui-tts, kokoro-tts all have this |
| Text-to-speech from file input | Users pipe files or pass paths; Kokoro supports TXT/EPUB/PDF, Piper reads stdin | MEDIUM | PDF extraction adds complexity; TXT and Markdown are straightforward |
| Voice selection | Every TTS CLI lets you pick a voice; users expect `--voice <name>` | LOW | For Chatter this means selecting a saved profile |
| Audio file output (WAV baseline) | WAV is the universal default; Piper outputs WAV, Coqui defaults to WAV | LOW | Model produces WAV natively; this is the zero-cost default |
| MP3 output format | Smaller files for sharing/storage; edge-tts defaults to MP3, Coqui supports it | LOW | Requires encoding step from WAV; well-understood problem |
| Language selection | Multilingual models require explicit language; Qwen3-TTS supports 11 languages | LOW | Flag-based selection; auto-detect is a differentiator |
| Progress feedback during inference | GPU inference takes seconds to minutes; users need to know it is working | MEDIUM | Progress bars, spinners, or X-of-Y counters; requires callbacks from PyO3 |
| Model selection | Users want to trade quality for speed; Qwen3-TTS has 0.6B and 1.7B variants | LOW | Simple flag: `--model small` vs `--model large` |
| Stdin pipe support | Unix philosophy; `cat text.txt \| chatter speak` is expected | LOW | Read from stdin when no file argument given; Piper and Kokoro both do this |
| Help and discoverability | `--help`, `--list-voices`, `--list-languages` are universal | LOW | Standard clap/CLI conventions |
| Error messages with guidance | When GPU is unavailable or model not downloaded, tell user what to do | LOW | Critical for GPU-dependent tools where setup failures are common |

### Differentiators (Competitive Advantage)

Features that set the product apart. Not required, but valuable.

| Feature | Value Proposition | Complexity | Notes |
|---------|-------------------|------------|-------|
| Voice design from natural language | No competitor offers "design a voice by describing it"; unique to Qwen3-TTS VoiceDesign model | MEDIUM | This is Chatter's killer feature. Users say "a warm female voice with a British accent" and get a reusable profile |
| Voice cloning from audio sample | Coqui supports this (6s clip), but most CLI tools do not; Bark explicitly does not support cloning | MEDIUM | Qwen3-TTS Base model handles this; requires reference MP3 input |
| Persistent voice profiles with metadata | No TTS CLI saves voice configs as reusable named profiles; users always pass flags each time | MEDIUM | Profiles stored in `~/.config/chatter/profiles/` with metadata and cached sample audio |
| Profile preview (cached sample audio) | Lets users audition voices without running inference; no competitor does this | LOW | Generate a sample on profile creation; store alongside metadata |
| Long document chunking with progress | Kokoro-tts splits documents into chunks; Chatter can do this with per-chunk progress | MEDIUM | Sentence-boundary splitting, sequential synthesis, progress per chunk |
| Smart text preprocessing | Strip markdown formatting, handle abbreviations, expand numbers; most CLI tools pass raw text to model | MEDIUM | Markdown/PDF parsing means cleaning formatting artifacts before synthesis |
| Voice blending weights | Kokoro-tts supports `voice1:0.7,voice2:0.3`; interesting but niche | HIGH | Not in Qwen3-TTS model natively; would require post-processing or mixing. Defer. |
| Auto language detection | Edge-tts requires explicit language; auto-detect from input text is more ergonomic | LOW | Qwen3-TTS supports auto-detect mode already; just expose it as default |

### Anti-Features (Commonly Requested, Often Problematic)

Features that seem good but create problems.

| Feature | Why Requested | Why Problematic | Alternative |
|---------|---------------|-----------------|-------------|
| Streaming audio playback | Users want to hear audio immediately without waiting for file write | Adds massive complexity (audio device management, platform-specific playback, buffering); v1 scope creep. Piper supports `--output-raw` piped to `aplay` but that is platform-specific. | Generate to file, let user play with their preferred player. Consider piping raw audio to stdout for power users. |
| Cloud/API integration (DashScope) | Some users want faster inference via cloud | Requires API keys, network dependency, billing management; contradicts local-first value prop | Stay local-only. Cloud can be a v2 consideration. |
| GUI or TUI interface | Visual voice browsing, waveform preview | Enormous scope increase; different product entirely | Good CLI with `--list-profiles` and `--preview` covers the need |
| SSML markup support | Fine-grained prosody control (pauses, emphasis, pitch) | Qwen3-TTS does not support SSML; implementing a parser for unsupported features wastes effort. Edge-tts removed SSML support. | Natural language prompts in voice design handle tone/style better for this model |
| Batch processing multiple files | Process entire directories of documents | Complicates error handling, progress reporting, and output naming; easy to script with shell loops | Single file per invocation; users wrap in `for f in *.txt; do chatter speak ...` |
| Voice profile sharing/export | Share voice configs with others | Profile portability across machines with different model versions is fragile; serialization format decisions | Local profiles only for v1. Export is a v2 feature once profile format stabilizes. |
| Real-time voice conversion | Transform one voice to another live | Completely different use case; latency-sensitive; not what Qwen3-TTS is designed for | Out of scope. Different tool. |
| Web server / API mode | Run as HTTP service for other apps | Scope creep into a different product category; Coqui and OpenEdAI-speech already serve this niche | Stay CLI. Users who need a server can wrap with a simple script. |

## Feature Dependencies

```
[Voice Design from Description]
    └──produces──> [Voice Profile]

[Voice Cloning from Audio]
    └──produces──> [Voice Profile]

[Voice Profile]
    └──required-by──> [Speech Generation]
    └──enhanced-by──> [Profile Preview / Cached Sample]

[Text Input (string/stdin)]
    └──feeds──> [Speech Generation]

[File Input (TXT/MD/PDF)]
    └──requires──> [Text Preprocessing]
                        └──feeds──> [Speech Generation]

[Speech Generation]
    └──requires──> [Model Loading]
    └──requires──> [Audio Encoding (WAV/MP3)]
    └──enhanced-by──> [Progress Feedback]
    └──enhanced-by──> [Long Document Chunking]

[Long Document Chunking]
    └──requires──> [Text Preprocessing]
    └──enhanced-by──> [Progress Feedback]
```

### Dependency Notes

- **Voice Profile required-by Speech Generation:** You must have a profile (designed or cloned) before you can generate speech. This means profile creation commands ship before or alongside the speak command.
- **File Input requires Text Preprocessing:** PDF and Markdown files contain formatting that must be stripped before sending to the model. TXT is pass-through.
- **Long Document Chunking requires Text Preprocessing:** Documents must be split at sentence boundaries after formatting is removed, not before.
- **Speech Generation requires Model Loading:** First inference has a cold-start cost (model download + GPU loading). Subsequent runs reuse cached models.
- **Profile Preview enhances Voice Profile:** Not a hard dependency, but creating the sample at profile-creation time means it is always available for `--preview`.

## MVP Definition

### Launch With (v1)

Minimum viable product -- what is needed to validate the concept.

- [ ] `chatter design` -- Create a voice profile from a natural language description (the unique differentiator)
- [ ] `chatter clone` -- Create a voice profile from a reference audio file
- [ ] `chatter speak` -- Generate speech from text/stdin using a saved profile
- [ ] `chatter speak --file <path>` -- Generate speech from TXT/Markdown file
- [ ] `chatter profiles list` -- List saved voice profiles
- [ ] `chatter profiles preview` -- Play or regenerate cached sample for a profile
- [ ] WAV and MP3 output formats
- [ ] Progress bars during model loading and inference
- [ ] Language selection flag (with auto-detect as default)
- [ ] Model size selection (0.6B / 1.7B)

### Add After Validation (v1.x)

Features to add once core is working.

- [ ] PDF file input -- Add when users request document formats beyond plain text
- [ ] Long document chunking with per-chunk progress -- Add when users report timeout or memory issues with large files
- [ ] Raw audio stdout piping (`--pipe-out`) -- Add when power users request Unix pipe integration
- [ ] OGG/FLAC output formats -- Add if users request beyond WAV/MP3
- [ ] `chatter profiles delete` and `chatter profiles info` -- Profile management ergonomics

### Future Consideration (v2+)

Features to defer until product-market fit is established.

- [ ] Voice profile export/import -- Wait until profile format stabilizes
- [ ] Batch file processing -- Wait until single-file pipeline is robust
- [ ] Subtitle/timing generation (SRT) -- Niche use case, add if audiobook users request it
- [ ] Shell completions (bash, zsh, fish) -- Quality of life, not essential

## Feature Prioritization Matrix

| Feature | User Value | Implementation Cost | Priority |
|---------|------------|---------------------|----------|
| Voice design from description | HIGH | MEDIUM | P1 |
| Voice cloning from audio | HIGH | MEDIUM | P1 |
| Speech generation (text/stdin) | HIGH | MEDIUM | P1 |
| Voice profile persistence | HIGH | LOW | P1 |
| WAV output | HIGH | LOW | P1 |
| MP3 output | HIGH | LOW | P1 |
| Progress bars | MEDIUM | MEDIUM | P1 |
| Language selection | MEDIUM | LOW | P1 |
| Model size selection | LOW | LOW | P1 |
| File input (TXT/MD) | MEDIUM | LOW | P1 |
| Profile listing | MEDIUM | LOW | P1 |
| Profile preview | MEDIUM | LOW | P2 |
| PDF file input | MEDIUM | MEDIUM | P2 |
| Long document chunking | MEDIUM | MEDIUM | P2 |
| Raw stdout piping | LOW | LOW | P2 |
| Auto language detection | LOW | LOW | P2 |
| Additional output formats | LOW | LOW | P3 |
| Profile export/import | LOW | MEDIUM | P3 |
| Subtitle generation | LOW | HIGH | P3 |
| Shell completions | LOW | LOW | P3 |

**Priority key:**
- P1: Must have for launch
- P2: Should have, add when possible
- P3: Nice to have, future consideration

## Competitor Feature Analysis

| Feature | Piper | edge-tts | Coqui TTS | Kokoro-tts | Bark | **Chatter** |
|---------|-------|----------|-----------|------------|------|-------------|
| Local inference | Yes (CPU) | No (cloud) | Yes (GPU) | Yes (GPU/CPU) | Yes (GPU) | **Yes (GPU)** |
| Voice cloning | No | No | Yes (6s clip) | No | No | **Yes (from audio)** |
| Voice design from text | No | No | No | No | No | **Yes (unique)** |
| Named voice profiles | No | No | No | No | No | **Yes (unique)** |
| Multi-language | Yes (per-model) | Yes (100+) | Yes (16+) | Yes (6) | Yes (13) | **Yes (11)** |
| Stdin pipe | Yes | No | Yes | Yes | No | **Yes** |
| File input (PDF/EPUB) | No | No | No | Yes | No | **TXT/MD (v1), PDF (v1.x)** |
| Progress feedback | No | No | No | No | No | **Yes** |
| Streaming playback | Yes (raw) | Yes | Yes (<200ms) | Yes | No | **No (file only, v1)** |
| Output formats | WAV | MP3 | WAV/MP3/OGG/FLAC | WAV/MP3 | WAV | **WAV/MP3** |
| Speed/rate control | No | Yes | No | Yes | No | **No (model-dependent)** |
| Subtitle generation | No | Yes (SRT) | No | No | No | **No (v2+)** |
| Voice blending | No | No | No | Yes | No | **No** |
| SSML support | No | Removed | No | No | No | **No** |

**Key competitive insight:** No existing TTS CLI tool combines voice design from natural language, voice cloning, and persistent named profiles. These three features together form Chatter's differentiation. Every competitor requires users to specify voices by opaque identifiers (voice IDs, model paths, preset numbers) on every invocation. Chatter lets users create memorable named profiles once and reuse them.

## Sources

- [Piper TTS (rhasspy/piper)](https://github.com/rhasspy/piper) -- archived, moved to OHF-Voice/piper1-gpl
- [edge-tts (rany2/edge-tts)](https://github.com/rany2/edge-tts) -- v7.2.8, Python CLI for Microsoft Edge TTS
- [Coqui TTS (coqui-ai/TTS)](https://github.com/coqui-ai/TTS) -- company closed Dec 2025, open source continues
- [Kokoro TTS (nazdridoy/kokoro-tts)](https://github.com/nazdridoy/kokoro-tts) -- v2.3.0, closest CLI feature set
- [Bark (suno-ai/bark)](https://github.com/suno-ai/bark) -- text-prompted generative audio, no voice cloning
- [Deepgram TTS chunking docs](https://developers.deepgram.com/docs/tts-text-chunking) -- chunking best practices
- [CLI UX progress patterns (Evil Martians)](https://evilmartians.com/chronicles/cli-ux-best-practices-3-patterns-for-improving-progress-displays) -- spinner, X-of-Y, progress bar patterns

---
*Feature research for: TTS CLI tools (local inference, voice profile management)*
*Researched: 2026-03-27*
