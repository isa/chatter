# Requirements: Chatter v1.1 ChatterBox Engine Support

## v1 Requirements

### Engine (ENG) -- Multi-engine infrastructure

| ID | Requirement | Priority |
|----|-------------|----------|
| ENG-01 | User can select TTS engine via `--engine qwen\|chatterbox` global flag or `CHATTER_ENGINE` env var | Must have |
| ENG-02 | Python bridge is refactored into a dispatcher that routes to engine-specific modules (`engines/qwen.py`, `engines/chatterbox.py`) | Must have |
| ENG-03 | Voice profiles carry an `engine` field; existing profiles default to `qwen` via `serde(default)`; engine-mismatch validation prevents using wrong profile with wrong engine | Must have |

### ChatterBox Inference (CB) -- Core ChatterBox functionality

| ID | Requirement | Priority |
|----|-------------|----------|
| CB-01 | User can clone a voice from an audio sample using ChatterBox (`chatter --engine chatterbox clone audio.wav`) | Must have |
| CB-02 | User can generate speech from text using a ChatterBox voice profile (`chatter --engine chatterbox generate "text" --profile name`) | Must have |
| CB-03 | Switching engines automatically unloads the previous engine's model to prevent OOM on 16GB Macs | Must have |
| CB-04 | ChatterBox Python dependencies are resolved without breaking Qwen3-TTS (shared venv with compatible pins, or engine-specific venvs as fallback) | Must have |

### Model Management (MDL) -- Download, cache, and diagnose

| ID | Requirement | Priority |
|----|-------------|----------|
| MDL-01 | `chatter model` command supports downloading and listing ChatterBox model variants (Original, Turbo, Multilingual) | Must have |
| MDL-02 | `chatter doctor` validates ChatterBox installation (package version, MPS/CUDA availability, disk space) | Must have |
| MDL-03 | Model download shows disk space requirements and warns before downloading large models | Must have |

### ChatterBox Features (FT) -- Engine-specific capabilities

| ID | Requirement | Priority |
|----|-------------|----------|
| FT-01 | User can control emotion intensity via `--exaggeration` (0.0-1.0) and `--cfg` flags when generating with ChatterBox Original | Should have |
| FT-02 | User can use paralinguistic tags (`[laugh]`, `[sigh]`, etc.) in text input for ChatterBox Turbo, with syntax validation | Should have |

## v2 (Deferred)

- Auto engine selection based on language/task
- Engine comparison mode (same text, both engines)
- Voice design for ChatterBox (not supported by the model)
- Real-time streaming
- Audio loudness normalization across engines

## Traceability

| Requirement | Phase | Status |
|-------------|-------|--------|
| ENG-01 | Phase 09 (code from Phase 06) | Complete |
| ENG-02 | Phase 09 (code from Phase 06) | Complete |
| ENG-03 | Phase 09 (code from Phase 06, needs TTY fix) | Complete |
| CB-01 | Phase 06 | Done |
| CB-02 | Phase 06 | Done |
| CB-03 | Phase 06 | Done |
| CB-04 | Phase 06 | Done |
| MDL-01 | Phase 07 | Done |
| MDL-02 | Phase 09 (partial from Phase 07, needs --fix repair) | Complete |
| MDL-03 | Phase 07 | Done |
| FT-01 | Phase 08 | Done |
| FT-02 | Phase 08 | Done |
