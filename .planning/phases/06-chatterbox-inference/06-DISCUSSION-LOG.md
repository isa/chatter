# Phase 06: ChatterBox Inference - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-03-29
**Phase:** 06-chatterbox-inference
**Areas discussed:** Backend strategy, Voice cloning model, Dependency install, Memory management

---

## Backend Strategy

### Primary backend path on Apple Silicon

| Option | Description | Selected |
|--------|-------------|----------|
| MLX-first with MPS fallback | Try mlx-audio community models first. If they work, clean MLX path. If not, fall back to MPS/PyTorch. | ✓ |
| MPS/PyTorch only | Skip MLX entirely. Use PyTorch with MPS backend. Proven but creates two-backend situation. | |
| You decide | Claude picks based on implementation. | |

**User's choice:** MLX-first with MPS fallback
**Notes:** Recommended approach accepted. Requires empirical validation at phase start.

### MLX fallback handling

| Option | Description | Selected |
|--------|-------------|----------|
| Silent fallback to MPS | Auto-switch to MPS on MLX failure. One-time info message. | ✓ |
| Explicit user choice | Show error, require --backend mps flag. | |
| Build both paths upfront | Implement MLX and MPS in parallel from start. | |

**User's choice:** Silent fallback to MPS
**Notes:** None.

### CUDA scope

| Option | Description | Selected |
|--------|-------------|----------|
| Apple Silicon first, CUDA later | Focus on primary hardware, CUDA as follow-up. | |
| Both Apple Silicon and CUDA | Full cross-platform from day one. | ✓ |
| You decide | Claude picks based on complexity. | |

**User's choice:** Both Apple Silicon and CUDA
**Notes:** User wants full platform support in this phase.

---

## Voice Cloning Model

### Profile storage for ChatterBox cloning data

| Option | Description | Selected |
|--------|-------------|----------|
| Reference audio only | Store ref_audio.wav. ChatterBox passes it at generate time. Simple, matches ChatterBox API. | ✓ |
| Reference audio + cached embeddings | Store audio plus pre-computed speaker embedding for faster repeated generation. | |
| You decide | Claude picks based on API support. | |

**User's choice:** Reference audio only
**Notes:** None.

### Clone command UX

| Option | Description | Selected |
|--------|-------------|----------|
| Same preview loop | Keep interactive flow: clone → preview → accept/retry/quit. Consistent across engines. | ✓ |
| Simplified: store and verify | Skip preview. Store audio, quick validation, save. | |
| You decide | Claude picks based on implementation fit. | |

**User's choice:** Same preview loop
**Notes:** None.

### Model variant selection

| Option | Description | Selected |
|--------|-------------|----------|
| Default variant, overridable | Sensible defaults (Original for English, Multilingual for non-English). Override with --variant. | ✓ |
| Always explicit | Require --variant on every ChatterBox command. | |
| You decide | Claude picks the strategy. | |

**User's choice:** Default variant, overridable
**Notes:** None.

---

## Dependency Install

### Installation method

| Option | Description | Selected |
|--------|-------------|----------|
| --no-deps + curated list | Install chatterbox-tts with --no-deps, curated requirements in repo. Avoids gradio bloat. | ✓ |
| Try shared venv first | pip install both normally. Fall back to --no-deps if conflicts. | |
| Engine-specific venvs | Separate venvs per engine. Guarantees no conflicts but doubles disk space. | |

**User's choice:** --no-deps + curated list
**Notes:** None.

### Installation timing

| Option | Description | Selected |
|--------|-------------|----------|
| Lazy on first --engine chatterbox | Install deps on first use. | |
| Eager during venv setup | Install both engines' deps at initial setup. | |
| Explicit via model download | User must run `chatter model download --engine chatterbox` first. | ✓ |

**User's choice:** Explicit via model download (free-text response)
**Notes:** User strongly prefers explicit behavior over lazy/automatic. If ChatterBox deps are not installed and user tries --engine chatterbox, show a clear error/warning directing them to `chatter model download --engine chatterbox`.

---

## Memory Management

### Unloading aggressiveness

| Option | Description | Selected |
|--------|-------------|----------|
| Full cleanup | unload_all_models() + del + gc.collect() + torch.mps.empty_cache() / torch.cuda.empty_cache() | ✓ |
| Basic unload only | Just unload_all_models(), rely on Python GC. | |
| You decide | Claude picks based on 16GB target. | |

**User's choice:** Full cleanup
**Notes:** None.

### Engine switch behavior

| Option | Description | Selected |
|--------|-------------|----------|
| Automatic on --engine flag | Detect mismatch, auto-unload previous engine. --engine flag is the intent signal. | ✓ |
| Warn and confirm | Detect mismatch, warn about memory, ask user to confirm. | |
| You decide | Claude picks the behavior. | |

**User's choice:** Automatic on --engine flag
**Notes:** None.

---

## Claude's Discretion

- MLX validation approach details
- Curated requirements list contents
- Error message wording
- Internal implementation details of ChatterBox engine functions

## Deferred Ideas

None — discussion stayed within phase scope.
