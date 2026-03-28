# Chatter

## What This Is

A Rust CLI tool that wraps Qwen3-TTS from Hugging Face to provide text-to-speech capabilities with voice profile management. Users can design custom voices from natural language descriptions, clone voices from audio samples, and generate speech from text or documents — all from the terminal with progress feedback.

## Core Value

Users can create reusable voice profiles and generate high-quality speech from text or documents without leaving the command line.

## Requirements

### Validated

- [x] Design voice profiles from natural language descriptions — Validated in Phase 2
- [x] Clone voice profiles from reference MP3 audio files — Validated in Phase 2
- [x] Generate speech from text input using a saved voice profile — Validated in Phase 2
- [x] Generate speech from file input (PDF/TXT/Markdown) using a saved voice profile — Validated in Phase 3
- [x] Save voice profiles with metadata and cached sample audio — Validated in Phase 2
- [x] Progress bars during all model inference operations — Validated in Phase 1-3
- [x] Language selection across all commands — Validated in Phase 1
- [x] MP3 output format for generated audio — Validated in Phase 2

### Active

(All v1 requirements validated)

### Out of Scope

- Cloud/DashScope API integration — keeping it local-only for simplicity
- Streaming audio playback — generate to file only for v1
- GUI or TUI — CLI only
- Voice profile sharing/export — local profiles only for v1
- Batch processing multiple files — single file per invocation for v1

## Context

- **Model**: Qwen3-TTS by Alibaba Qwen team, accessed via `qwen-tts` Python package (`pip install -U qwen-tts`)
- **Model variants**: VoiceDesign 1.7B (design), Base 0.6B/1.7B (clone), CustomVoice 0.6B/1.7B (generate)
- **Python integration**: PyO3 to embed Python in Rust — the `qwen_tts` package is Python-only
- **Inference**: Local GPU required, PyTorch with bfloat16, Flash Attention supported
- **Profile storage**: `~/.config/chatter/profiles/` following XDG Base Directory spec
- **Reference HF Space**: https://huggingface.co/spaces/Qwen/Qwen3-TTS

## Constraints

- **Tech stack**: Rust CLI with PyO3 for Python interop — required because model is Python-only
- **Hardware**: CUDA-capable GPU required for local inference
- **Dependencies**: Python 3.12+ runtime with `qwen-tts` package installed
- **Audio**: MP3 output (requires encoding from WAV produced by model)

## Key Decisions

| Decision | Rationale | Outcome |
|----------|-----------|---------|
| PyO3 over subprocess | Tighter integration, better error handling, progress callbacks | — Pending |
| Local-only inference | Simplicity, no API keys needed, full control | — Pending |
| XDG profile storage | Community standard for config/data on Linux/macOS | — Pending |
| MP3 output format | Smaller file sizes than WAV | — Pending |
| Metadata + cached sample in profiles | Allows previewing voices without re-running model | — Pending |

## Evolution

This document evolves at phase transitions and milestone boundaries.

**After each phase transition** (via `/gsd:transition`):
1. Requirements invalidated? → Move to Out of Scope with reason
2. Requirements validated? → Move to Validated with phase reference
3. New requirements emerged? → Add to Active
4. Decisions to log? → Add to Key Decisions
5. "What This Is" still accurate? → Update if drifted

**After each milestone** (via `/gsd:complete-milestone`):
1. Full review of all sections
2. Core Value check — still the right priority?
3. Audit Out of Scope — reasons still valid?
4. Update Context with current state

---
*Last updated: 2026-03-28 after Phase 3 completion — all v1 phases complete*
