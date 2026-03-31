# Project: Chatter

## Core Value

Users can create reusable voice profiles and generate high-quality speech from text or documents without leaving the command line.

## Current state (shipped)

**v1.1 ChatterBox Engine Support** (2026-03-31) — Dual-engine CLI: Qwen3-TTS and ChatterBox with voice cloning, model management, engine-specific controls (`--exaggeration`, `--cfg`, paralinguistic tags), TTY-aware engine/profile validation, and curated `doctor --fix` install plus hardware visibility.

**v1.0 MVP** — Homebrew-style distribution, managed Python venv, Qwen3-TTS profiles, generate from text/documents, MP3 output.

## Next milestone

Planning not started. Use `/gsd-new-milestone` when ready to define v1.2+ scope.

## Constraints

- **Tech stack**: Rust CLI with PyO3 for Python interop
- **Hardware**: Apple Silicon (MLX/MPS) or CUDA-capable GPU required
- **Distribution**: `brew install chatter` must work out of the box
- **Python**: Managed venv at `~/.local/share/chatter/venv/` with auto-setup
- **Dependencies**: Python 3.12+, `qwen-tts` and `chatterbox-tts` packages
- **Audio**: MP3 output (WAV from model, encoded in Rust)
- **Memory**: Must handle engine switching on 16GB Macs without OOM

## Key Decisions

| Date | Decision | Context |
|------|----------|---------|
| 2026-03-29 | v1.1 starts at Phase 04 | Continues numbering from v1.0 (Phases 01-03) |
| 2026-03-29 | Five intended workstreams for v1.1 | Research: abstraction → CLI → inference → management → features (04/05 later superseded by 06 inline) |
| 2026-03-29 | Dispatcher pattern for multi-engine | Bridge routes to engine modules |
| 2026-03-29 | `serde(default)` for profile backward compat | Existing profiles default to `qwen` |
| 2026-03-31 | Phases 04/05 superseded by Phase 06 | Engine abstraction and CLI routing landed with ChatterBox inference |
| 2026-03-31 | Phase 09 closed milestone-audit gaps | TTY-aware mismatch errors, curated ChatterBox `doctor --fix`, hardware checks, dead-code cleanup |

## Open validation (not blocking ship)

- MLX community ChatterBox models: exercise in the field
- Transformers/qwen vs chatterbox CUDA pin conflicts: watch on NVIDIA setups
- ChatterBox on Python 3.12: continue to validate in user environments

## References

- Research: `.planning/research/SUMMARY.md`
- Roadmap: `.planning/ROADMAP.md`
- Shipped requirements (v1.1): `.planning/milestones/v1.1-REQUIREMENTS.md`
- Milestone log: `.planning/MILESTONES.md`

---
*Last updated: 2026-03-31 after v1.1 milestone archive*
