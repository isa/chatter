# Milestones

## v1.1 ChatterBox Engine Support (Shipped: 2026-03-31)

**Phases completed:** 6 phases, 9 plans, 13 tasks

**Key accomplishments:**

- ChatterBox dependency pipeline, engine-aware model download, variant CLI flag, set_variant bridge, and profile variant persistence
- Full ChatterBox engine with MLX-first backend detection, variant-aware inference (Original/Turbo/Multilingual), MPS-safe PyTorch loading, and memory-safe engine switching
- Engine-grouped model listing with ChatterBox variant labels and disk space pre-check before large downloads
- Doctor command validates ChatterBox installation alongside Qwen3-TTS with per-engine sections and extended --fix for both engines
- Wired exaggeration and cfg_weight parameters through full stack: Python ChatterBox engine, dispatcher, Rust PyO3 bridge, and CLI --exaggeration/--cfg flags with backward-compatible 0.5 defaults
- Engine-specific flag gating and paralinguistic tag validation for ChatterBox Turbo (Phase 08 Plan 02)
- Milestone gap closure: TTY-aware engine/profile mismatch errors, curated `doctor --fix` ChatterBox install and model download, hardware visibility, dead-code cleanup (Phase 09)

**Audit note:** Initial v1.1 milestone audit (`milestones/v1.1-MILESTONE-AUDIT.md`) captured pre-closure gaps; Phase 09 addressed the integration items called out there.

---
