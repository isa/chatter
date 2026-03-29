# Roadmap: Chatter

## Milestones

- **v1.0 MVP** - Phases 01-03 (shipped)
- **v1.1 ChatterBox Engine Support** - Phases 04-08 (in progress)

## Phases

<details>
<summary>v1.0 MVP (Phases 01-03) - SHIPPED</summary>

Qwen3-TTS integration with voice design, voice cloning, speech generation, and model management. Delivered as a Homebrew-installable Rust CLI with managed Python venv.

</details>

### v1.1 ChatterBox Engine Support

**Milestone Goal:** Add ChatterBox as a second TTS engine with full voice cloning, model management, and engine-specific controls -- without breaking existing Qwen3-TTS functionality.

- [ ] **Phase 04: Engine Abstraction** - Refactor Python bridge into multi-engine dispatcher
- [ ] **Phase 05: CLI Engine Routing** - Add --engine flag and profile engine tagging in Rust
- [x] **Phase 06: ChatterBox Inference** - Implement ChatterBox voice cloning and speech generation (2026-03-29)
- [ ] **Phase 07: Model Management and Diagnostics** - Extend model and doctor commands for ChatterBox
- [ ] **Phase 08: ChatterBox Controls** - Expose emotion, exaggeration, and paralinguistic tag features

## Phase Details

### Phase 04: Engine Abstraction
**Goal**: The Python bridge supports multiple engine modules without changing any existing behavior
**Depends on**: Phase 03 (v1.0 complete)
**Requirements**: ENG-02
**Success Criteria** (what must be TRUE):
  1. All existing `chatter` commands (design, clone, generate, model, doctor) work identically after the refactor -- no user-visible behavior change
  2. Python bridge code is organized into `engines/qwen.py` (extracted from monolith) and a stub `engines/chatterbox.py`, with `chatter_bridge.py` acting as a dispatcher
  3. `set_engine("qwen")` succeeds and routes all calls through the qwen engine module
  4. Each engine module owns its own backend detection (MLX vs MPS vs CUDA) rather than relying on a global detector
**Plans**: 1 plan
Plans:
- [ ] 04-01-PLAN.md -- Refactor Python bridge into engine package and update Rust deployment

### Phase 05: CLI Engine Routing
**Goal**: Users can select which TTS engine to use, and profiles know which engine created them
**Depends on**: Phase 04
**Requirements**: ENG-01, ENG-03
**Success Criteria** (what must be TRUE):
  1. User can pass `--engine chatterbox` or `--engine qwen` as a global flag, or set `CHATTER_ENGINE=chatterbox` env var, and the bridge receives the correct engine selection
  2. Existing voice profiles (created before v1.1) load without error and default to `engine: "qwen"`
  3. Running `chatter --engine chatterbox design` exits with a clear error message explaining voice design is not available for ChatterBox
  4. Running `chatter --engine chatterbox generate --profile qwen-profile` exits with a clear error explaining engine mismatch between profile and selected engine
**Plans**: 1 plan
Plans:
- [ ] 05-01-PLAN.md -- Add --engine flag, profile engine tagging, and validation guards

### Phase 06: ChatterBox Inference
**Goal**: Users can clone voices and generate speech using ChatterBox models
**Depends on**: Phase 05
**Requirements**: CB-01, CB-02, CB-03, CB-04
**Success Criteria** (what must be TRUE):
  1. User can run `chatter --engine chatterbox clone reference.wav --name myvoice` and get a working ChatterBox voice profile
  2. User can run `chatter --engine chatterbox generate "Hello world" --profile myvoice` and get an MP3 audio file with recognizable speech
  3. Switching from `--engine qwen` to `--engine chatterbox` (or vice versa) between commands does not cause out-of-memory crashes on a 16GB Mac
  4. ChatterBox Python dependencies install correctly into the managed venv without breaking existing Qwen3-TTS functionality
  5. `chatter --engine chatterbox` works on Apple Silicon (via MLX community models or MPS fallback)
**Plans**: 2 plans
Plans:
- [x] 06-01-PLAN.md -- Curated dependency installation pipeline and engine-aware model download
- [x] 06-02-PLAN.md -- ChatterBox engine module implementation and memory-safe engine switching

### Phase 07: Model Management and Diagnostics
**Goal**: Users can download, list, and diagnose ChatterBox models through existing CLI commands
**Depends on**: Phase 06
**Requirements**: MDL-01, MDL-02, MDL-03
**Success Criteria** (what must be TRUE):
  1. `chatter model list` shows both Qwen3-TTS and ChatterBox cached models with their sizes
  2. `chatter model download --engine chatterbox` downloads ChatterBox model variants (Original, Turbo, Multilingual) and shows disk space required before starting
  3. `chatter doctor` validates ChatterBox installation status (package installed, compatible hardware detected, sufficient disk space) alongside existing Qwen3-TTS checks
**Plans**: 2 plans
Plans:
- [ ] 07-01-PLAN.md -- Engine-grouped model listing and disk space pre-check for ChatterBox downloads
- [ ] 07-02-PLAN.md -- Extend doctor command with ChatterBox diagnostics and --fix support

### Phase 08: ChatterBox Controls
**Goal**: Users can leverage ChatterBox-specific audio generation features not available in Qwen3-TTS
**Depends on**: Phase 06
**Requirements**: FT-01, FT-02
**Success Criteria** (what must be TRUE):
  1. User can pass `--exaggeration 0.7` when generating with ChatterBox Original and hear a perceptible difference in expressiveness compared to the default
  2. User can pass `--cfg` to control classifier-free guidance weight for ChatterBox Original
  3. User can include `[laugh]` or `[sigh]` tags in text input for ChatterBox Turbo and the generated audio contains the corresponding non-speech sound
  4. Invalid paralinguistic tags produce a clear validation error before inference starts, not a cryptic Python traceback
**Plans**: TBD
**UI hint**: no

## Progress

**Execution Order:** 04 -> 05 -> 06 -> 07 -> 08

Note: Phase 07 and Phase 08 both depend on Phase 06 but are independent of each other. They can execute in either order.

| Phase | Milestone | Plans Complete | Status | Completed |
|-------|-----------|----------------|--------|-----------|
| 04. Engine Abstraction | v1.1 | 0/1 | Planning complete | - |
| 05. CLI Engine Routing | v1.1 | 0/1 | Planning complete | - |
| 06. ChatterBox Inference | v1.1 | 2/2 | Complete | 2026-03-29 |
| 07. Model Management and Diagnostics | v1.1 | 0/2 | Planning complete | - |
| 08. ChatterBox Controls | v1.1 | 0/TBD | Not started | - |
