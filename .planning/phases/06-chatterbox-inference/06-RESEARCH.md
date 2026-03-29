# Phase 6: ChatterBox Inference - Research

**Researched:** 2026-03-29
**Domain:** ChatterBox TTS engine implementation (Python inference via PyO3), Apple Silicon MLX/MPS, CUDA
**Confidence:** MEDIUM-HIGH

## Summary

Phase 6 implements the ChatterBox engine module (`engines/chatterbox.py`) to fill in the stub created in Phase 4. The codebase already has the dispatcher pattern, engine routing, profile engine tagging, and all Rust bridge functions in place -- this phase is purely Python-side implementation plus a curated dependency installation strategy.

ChatterBox offers three model variants via different Python classes: `ChatterboxTTS` (Original, 500M params, emotion controls), `ChatterboxTurboTTS` (Turbo, 350M params, paralinguistic tags, fastest), and `ChatterboxMultilingualTTS` (500M params, 23 languages). All three output 24kHz audio tensors via `model.generate()` and accept voice cloning via `audio_prompt_path` parameter. On Apple Silicon, MLX community models exist for all variants via `mlx-audio` (`mlx-community/chatterbox-fp16`, `mlx-community/chatterbox-turbo-fp16`), providing the preferred zero-torch path. MPS fallback via PyTorch has known stability issues and may require CPU-first loading with selective `.to("mps")`. On CUDA, the standard PyTorch path works directly.

**Primary recommendation:** Implement MLX-first with MPS/CPU fallback on Apple Silicon. Start with an MLX validation task before writing inference code. Use `--no-deps` installation with a curated requirements file checked into the repo. Memory management via full cleanup on engine switch is critical for 16GB Mac support.

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions
- **D-01:** MLX-first with MPS/PyTorch fallback on Apple Silicon. Try `mlx-audio` community models first. If validation fails, silently fall back to MPS with a one-time info message.
- **D-02:** Both Apple Silicon and CUDA must be supported in this phase.
- **D-03:** Backend detection is per-engine. ChatterBox module owns its own `detect_backend()`.
- **D-04:** ChatterBox profiles store reference audio only (`ref_audio.wav`). No pre-computed voice prompts.
- **D-05:** Clone command uses the same preview-listen-retry loop as Qwen for consistent UX.
- **D-06:** Default model variant with override. Original for English, Multilingual for non-English. User can override with `--variant turbo|original|multilingual`. Profile stores which variant was used.
- **D-07:** Install `chatterbox-tts` with `--no-deps` plus a curated requirements list checked into the repo.
- **D-08:** Explicit install via `chatter model download --engine chatterbox`. Show clear error if deps missing.
- **D-09:** Full cleanup on engine switch: `unload_all_models()` + `del` references + `gc.collect()` + `torch.mps.empty_cache()` / `torch.cuda.empty_cache()`.
- **D-10:** Engine switching is automatic on `--engine` flag. No confirmation prompt.

### Claude's Discretion
- MLX validation approach (how to test community models at phase start)
- Curated requirements list contents for `chatterbox-tts --no-deps`
- Error message wording for missing ChatterBox deps
- ChatterBox `generate_speech()` and `voice_clone_from_audio()` internal implementation details

### Deferred Ideas (OUT OF SCOPE)
None -- discussion stayed within phase scope.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| CB-01 | Voice cloning from audio sample using ChatterBox | ChatterBox API accepts `audio_prompt_path` for all 3 variants. MLX path uses `ref_audio` param. Profile stores `ref_audio.wav` only (D-04). |
| CB-02 | Speech generation from text using ChatterBox profile | `model.generate(text=..., audio_prompt_path=...)` returns tensor at 24kHz. Same return format `(list_of_floats, sample_rate)` as Qwen engine. |
| CB-03 | Engine switching unloads previous engine to prevent OOM | Full cleanup pattern documented: del models + gc.collect() + device cache clear. Qwen's `unload_all_models()` is the reference. |
| CB-04 | ChatterBox deps resolved without breaking Qwen3-TTS | `--no-deps` install with curated requirements avoids gradio bloat and version conflicts. Transformers conflict (5.2.0 vs 4.57.3) sidestepped on Apple Silicon via MLX path. |
</phase_requirements>

## Standard Stack

### Core (no new Rust crates)

No changes to Cargo.toml. All ChatterBox integration is Python-side.

| Technology | Version | Purpose | Why Standard |
|------------|---------|---------|--------------|
| chatterbox-tts | 0.1.7 | ChatterBox model inference | Official Resemble AI package. Provides `ChatterboxTTS`, `ChatterboxTurboTTS`, `ChatterboxMultilingualTTS`. |
| mlx-audio | >=0.2.8 | MLX inference for ChatterBox on Apple Silicon | Community-maintained MLX conversions. Provides `load_model()` + `generate_audio()` API. Zero PyTorch dependency path. |
| torch | 2.6.0 | PyTorch runtime (MPS/CUDA fallback) | Required for non-MLX paths. Already in venv for Qwen. |
| torchaudio | 2.6.0 | Audio I/O for PyTorch path | Already in venv for Qwen. Used for `ta.save()` pattern. |

### Curated Requirements for `--no-deps` Install

These are the runtime dependencies of `chatterbox-tts` 0.1.7 minus gradio and resemble-perth (watermarking, not needed for inference). This list should be checked into the repo as `requirements/chatterbox.txt`:

```
chatterbox-tts==0.1.7
# Core deps (install with --no-deps, then install these)
numpy>=1.24.0,<2.0.0
librosa==0.11.0
s3tokenizer
torch==2.6.0
torchaudio==2.6.0
transformers==5.2.0
diffusers==0.29.0
conformer==0.3.2
safetensors==0.5.3
pykakasi==2.3.0
pyloudnorm
omegaconf
spacy-pkuseg
```

**Key exclusions:**
- `gradio==6.8.0` -- Web UI, ~200MB+ transitive deps, not needed for CLI inference
- `resemble-perth @ git+https://...` -- Watermarking library. Git URL dependency complicates Homebrew. Not needed for inference.

**CUDA vs Apple Silicon note:** On Apple Silicon with MLX, only `mlx-audio` is needed (no torch/chatterbox-tts). The curated list above is for MPS fallback and CUDA paths.

### MLX Community Models Available

| Model ID | Variant | Size | Status |
|----------|---------|------|--------|
| `mlx-community/chatterbox-fp16` | Original | 3.19 GB | Available, UNVALIDATED |
| `mlx-community/chatterbox-turbo-fp16` | Turbo | 2.99 GB | Available, UNVALIDATED |
| `mlx-community/Chatterbox-TTS-fp16` | Original (alt) | ~3 GB | Available, UNVALIDATED |
| `mlx-community/chatterbox-4bit` | Original (4-bit) | ~1 GB | Available, UNVALIDATED |

**No MLX multilingual model found.** This means Multilingual variant must fall back to MPS/CPU on Apple Silicon.

## Architecture Patterns

### ChatterBox Engine Module Structure

Follow the exact pattern from `engines/qwen.py`:

```
chatter_bridge/
  engines/
    chatterbox.py    # Implements all engine interface functions
```

### Pattern 1: MLX-First Backend Detection (D-01, D-03)

```python
# Source: Pattern derived from qwen.py detect_backend() + D-01 constraint
_backend_cache = None
_models = {}
_variant = "original"  # or "turbo" or "multilingual"

def detect_backend():
    """Return 'mlx', 'mps', 'cuda', or 'cpu'."""
    global _backend_cache
    if _backend_cache is not None:
        return _backend_cache

    # MLX first (Apple Silicon preferred)
    if _variant != "multilingual":  # No MLX multilingual model exists
        try:
            import mlx.core as mx
            if mx.metal.is_available():
                _backend_cache = "mlx"
                return "mlx"
        except ImportError:
            pass

    import torch
    if torch.cuda.is_available():
        _backend_cache = "cuda"
        return "cuda"
    if hasattr(torch.backends, "mps") and torch.backends.mps.is_available():
        _backend_cache = "mps"
        return "mps"
    _backend_cache = "cpu"
    return "cpu"
```

### Pattern 2: ChatterBox PyTorch Model Loading (MPS Safe Pattern)

The MPS path has known stability issues. Use CPU-first loading then selective `.to()`:

```python
# Source: Jimmi42/chatterbox-tts-apple-silicon-code (MPS adaptation)
def _load_pytorch_model(variant, device):
    """Load ChatterBox via PyTorch with safe device mapping."""
    if variant == "turbo":
        from chatterbox.tts_turbo import ChatterboxTurboTTS
        model = ChatterboxTurboTTS.from_pretrained("cpu")
    elif variant == "multilingual":
        from chatterbox.mtl_tts import ChatterboxMultilingualTTS
        model = ChatterboxMultilingualTTS.from_pretrained("cpu")
    else:  # original
        from chatterbox.tts import ChatterboxTTS
        model = ChatterboxTTS.from_pretrained("cpu")

    if device != "cpu":
        # Move submodels to device (MPS-safe pattern)
        model.t3 = model.t3.to(device)
        model.s3gen = model.s3gen.to(device)
        model.ve = model.ve.to(device)

    return model
```

### Pattern 3: ChatterBox Generate Speech (D-04)

ChatterBox passes reference audio directly at inference time -- no pre-computed prompts:

```python
# Source: Official ChatterBox GitHub README
def generate_speech(text, language, profile_dir, ref_text="", **kwargs):
    """Generate speech with ChatterBox using stored ref_audio."""
    backend = detect_backend()
    ref_audio_path = os.path.join(profile_dir, "ref_audio.wav")

    if backend == "mlx":
        from mlx_audio.tts.utils import load_model
        from mlx_audio.tts.generate import generate_audio
        model_id = _mlx_model_id()
        model = _ensure_mlx_model(model_id)
        # mlx-audio generate returns results with .audio attribute
        results = list(model.generate(
            text=text,
            ref_audio=ref_audio_path,
        ))
        audio = np.array(results[0].audio, dtype=np.float32)
        return audio.tolist(), 24000
    else:
        model = _ensure_pytorch_model()
        # All variants accept audio_prompt_path for voice cloning
        if _variant == "multilingual":
            wav = model.generate(
                text=text,
                audio_prompt_path=ref_audio_path,
                language_id=language,
            )
        elif _variant == "original":
            wav = model.generate(
                text=text,
                audio_prompt_path=ref_audio_path,
                exaggeration=0.5,
                cfg_weight=0.5,
            )
        else:  # turbo
            wav = model.generate(
                text=text,
                audio_prompt_path=ref_audio_path,
            )
        audio = wav.squeeze().cpu().numpy().astype(np.float32)
        return audio.tolist(), 24000
```

### Pattern 4: Memory-Safe Engine Switching (D-09, D-10)

```python
# Source: Pattern from qwen.py unload_all_models() + D-09 requirements
def unload_all_models():
    """Release all cached models to free memory."""
    global _models, _backend_cache
    for key in list(_models.keys()):
        del _models[key]
    _models.clear()

    import gc
    gc.collect()

    # Device-specific cache clearing
    try:
        import torch
        if torch.cuda.is_available():
            torch.cuda.empty_cache()
        if hasattr(torch.backends, "mps") and torch.backends.mps.is_available():
            torch.mps.empty_cache()
    except ImportError:
        pass  # MLX path has no torch

    # Reset backend cache so re-detection happens on next call
    _backend_cache = None
```

### Pattern 5: Dependency Check Before Inference (D-08)

```python
def _check_deps():
    """Verify ChatterBox deps are installed. Raise clear error if not."""
    try:
        import chatterbox
    except ImportError:
        raise ImportError(
            "ChatterBox is not installed. Run:\n"
            "  chatter model download --engine chatterbox\n"
            "to install ChatterBox models and dependencies."
        )
```

### Anti-Patterns to Avoid

- **Loading both engines simultaneously:** Always `unload_all_models()` from the previous engine before loading the new one. The dispatcher in `__init__.py` should handle this in `set_engine()`.
- **Using `from_pretrained(device="mps")` directly:** MPS has CUDA deserialization issues. Always load to CPU first, then `.to(device)`.
- **Installing chatterbox-tts with full deps:** Pulls in gradio (200MB+) and resemble-perth (git URL). Always `--no-deps`.
- **Caching backend across variant changes:** If variant changes from "turbo" to "multilingual", backend may need to change (MLX -> MPS) since no MLX multilingual exists.

### Recommended Project Structure Changes

```
chatter_bridge/
  engines/
    chatterbox.py      # IMPLEMENT (currently stub)
requirements/
  chatterbox.txt       # NEW: curated deps for --no-deps install
```

No changes to:
- `src/bridge/inference.rs` (already dispatches to active engine)
- `src/commands/clone.rs` (already engine-aware)
- `src/commands/generate.rs` (already engine-aware)

Changes needed:
- `src/bridge/venv.rs` -- Add ChatterBox dependency installation logic
- `src/commands/model.rs` -- Support `--engine chatterbox` for download command

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| ChatterBox inference | Custom model loading | `chatterbox.tts.ChatterboxTTS.from_pretrained()` | Official API handles checkpoint loading, tokenizer setup, internal component wiring |
| MLX inference | Direct MLX operations | `mlx_audio.tts.utils.load_model()` + `model.generate()` | Handles model format conversion, memory management, Apple Silicon optimization |
| Audio resampling | Manual resampling | `torchaudio` or `librosa` (already deps) | ChatterBox outputs 24kHz matching Qwen -- no resampling needed |
| Watermark detection | resemble-perth integration | Skip entirely | Watermarking is automatic in model output. Detection/verification is out of scope for CLI. |

## Common Pitfalls

### Pitfall 1: MPS Tensor Deserialization Crash
**What goes wrong:** `ChatterboxTTS.from_pretrained("mps")` fails with CUDA-related RuntimeError because model checkpoints contain CUDA tensor metadata.
**Why it happens:** Model weights were saved on CUDA. PyTorch's `torch.load()` tries to deserialize to the original device.
**How to avoid:** Always `from_pretrained("cpu")` then selectively `.to(device)` for t3, s3gen, ve submodels.
**Warning signs:** RuntimeError mentioning CUDA device when running on MPS.

### Pitfall 2: Transformers Version Conflict (CUDA Only)
**What goes wrong:** `chatterbox-tts` requires `transformers==5.2.0`; `qwen-tts` requires `transformers==4.57.3`. Cannot coexist.
**Why it happens:** Both packages pin exact transformers versions.
**How to avoid:** On Apple Silicon, use MLX for both engines (no transformers needed for MLX path). On CUDA, `--no-deps` install and test if ChatterBox works with 4.57.3 (likely yes for inference). If not, engine-specific venvs.
**Warning signs:** ImportError or version mismatch warnings on `import chatterbox`.

### Pitfall 3: OOM on 16GB Mac from Dual Engine Loading
**What goes wrong:** Loading ChatterBox after Qwen (or vice versa) without cleanup causes OOM.
**Why it happens:** ChatterBox Original is ~4GB in memory, Qwen models are ~3-4GB. Combined exceeds 16GB with OS overhead.
**How to avoid:** Full cleanup sequence per D-09 in `set_engine()` before loading new engine. The dispatcher (`__init__.py set_engine()`) should call `unload_all_models()` on the current engine before switching.
**Warning signs:** macOS memory pressure warnings, process killed by OS.

### Pitfall 4: No MLX Multilingual Model
**What goes wrong:** Attempting to use MLX path for `--variant multilingual` fails because no MLX community model exists.
**Why it happens:** Only Original and Turbo have been converted to MLX format by the community.
**How to avoid:** Variant-aware backend detection. If variant is "multilingual", skip MLX detection and fall back to MPS/CUDA/CPU.
**Warning signs:** Model not found error from `load_model("mlx-community/chatterbox-multilingual-fp16")`.

### Pitfall 5: gradio Installation Bloat
**What goes wrong:** `pip install chatterbox-tts` pulls in 200MB+ of gradio transitive dependencies.
**Why it happens:** `chatterbox-tts` declares `gradio==6.8.0` as a hard dependency in pyproject.toml.
**How to avoid:** Always `pip install --no-deps chatterbox-tts` then install curated requirements list.
**Warning signs:** Extremely long install time, hundreds of packages being installed.

### Pitfall 6: ChatterBox Python 3.12 Compatibility
**What goes wrong:** ChatterBox was developed and tested on Python 3.11. Some dependencies (spacy-pkuseg) may have build issues on 3.12.
**Why it happens:** Version pins in pyproject.toml target 3.11 ecosystem.
**How to avoid:** Test the curated requirements on 3.12 early. `pkuseg` may need `--no-build-isolation` flag.
**Warning signs:** Build failures during `pip install`, missing C extensions.

## Code Examples

### ChatterBox Original -- Voice Cloning + Generate (PyTorch)
```python
# Source: https://github.com/resemble-ai/chatterbox README
from chatterbox.tts import ChatterboxTTS
import torchaudio as ta

model = ChatterboxTTS.from_pretrained(device="cuda")  # or "cpu" for MPS-safe
wav = model.generate(
    text="Hello, this is a cloned voice.",
    audio_prompt_path="reference.wav",
    exaggeration=0.5,   # 0.0-1.0 emotion intensity
    cfg_weight=0.5,      # classifier-free guidance
)
ta.save("output.wav", wav, model.sr)  # model.sr == 24000
```

### ChatterBox Turbo -- With Paralinguistic Tags (PyTorch)
```python
# Source: https://github.com/resemble-ai/chatterbox README
from chatterbox.tts_turbo import ChatterboxTurboTTS

model = ChatterboxTurboTTS.from_pretrained(device="cuda")
wav = model.generate(
    text="Oh wow! [laugh] That's amazing. [sigh] Anyway...",
    audio_prompt_path="reference.wav",
)
# model.sr == 24000
```

### ChatterBox Multilingual (PyTorch)
```python
# Source: https://github.com/resemble-ai/chatterbox README
from chatterbox.mtl_tts import ChatterboxMultilingualTTS

model = ChatterboxMultilingualTTS.from_pretrained(device="cuda")
wav = model.generate(
    text="Bonjour, comment allez-vous?",
    audio_prompt_path="reference.wav",
    language_id="fr",
)
# model.sr == 24000
```

### MLX Path -- Voice Cloning (Apple Silicon)
```python
# Source: https://huggingface.co/mlx-community/chatterbox-fp16
from mlx_audio.tts.utils import load_model
import numpy as np

model = load_model("mlx-community/chatterbox-fp16")
results = list(model.generate(
    text="Hello, this is a cloned voice.",
    ref_audio="reference.wav",
))
audio = np.array(results[0].audio, dtype=np.float32)
# 24000 Hz sample rate
```

### Engine-Aware Unload in Dispatcher
```python
# Pattern for __init__.py set_engine() enhancement
def set_engine(name):
    global _active_engine, _active_engine_name
    from chatter_bridge.engines import AVAILABLE_ENGINES
    if name not in AVAILABLE_ENGINES:
        raise ValueError(f"Unknown engine: {name!r}")

    # D-09/D-10: Unload current engine before switching
    if _active_engine is not None and _active_engine_name != name:
        try:
            _active_engine.unload_all_models()
        except Exception:
            pass  # Best-effort cleanup

    _active_engine = importlib.import_module(AVAILABLE_ENGINES[name])
    _active_engine_name = name
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Single `chatter_bridge.py` monolith | Dispatcher + `engines/` modules | Phase 04 (completed) | Engine module pattern ready for ChatterBox |
| Global `detect_backend()` | Per-engine `detect_backend()` | Phase 04 (completed) | Each engine chooses its own backend |
| No engine field in profiles | `engine` field with `serde(default)` | Phase 05 (completed) | ChatterBox profiles carry engine identity |
| Qwen-only model download | Needs engine-aware download | This phase | `chatter model download --engine chatterbox` |

## Open Questions

1. **MLX Community Model Quality**
   - What we know: Models exist on HuggingFace (`mlx-community/chatterbox-fp16`, `chatterbox-turbo-fp16`). They were converted using mlx-audio 0.2.8.
   - What's unclear: Whether voice cloning quality matches PyTorch. Whether the `generate()` API on these models supports all parameters (ref_audio, lang_code).
   - Recommendation: First task in phase must be empirical validation. Load model, generate with ref_audio, compare to PyTorch output. If quality is acceptable, use MLX path. If not, fall to MPS.

2. **Transformers Coexistence on CUDA**
   - What we know: ChatterBox pins 5.2.0, Qwen pins 4.57.3. Hard conflict.
   - What's unclear: Whether ChatterBox actually breaks with transformers 4.57.3 at runtime (it may only use basic features).
   - Recommendation: Test ChatterBox inference with transformers 4.57.3 during dependency setup. If it works, pin 4.57.3 in curated list. If not, document engine-specific venv strategy as future work.

3. **spacy-pkuseg on Python 3.12**
   - What we know: ChatterBox depends on spacy-pkuseg (Chinese tokenization). Build may fail on 3.12.
   - What's unclear: Whether spacy-pkuseg is actually needed for English/non-Chinese inference.
   - Recommendation: Test without it first. If ChatterBox imports fine without it (for non-Chinese text), exclude from curated list. If required, try `--no-build-isolation` or pin a 3.12-compatible version.

4. **ChatterBox Variant Model Loading Time**
   - What we know: Original ~500M params (~4GB), Turbo ~350M params (~2GB), Multilingual ~500M params (~4GB).
   - What's unclear: Actual load time on Apple Silicon. Whether switching between variants within ChatterBox engine requires full unload/reload.
   - Recommendation: Cache one model at a time. Variant switch triggers unload + reload.

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| Python 3.12 | PyO3 bridge | Yes | 3.12.3 | -- |
| Apple Silicon (arm64) | MLX inference | Yes | M-series (128GB) | MPS/CPU |
| mlx-audio | MLX ChatterBox path | Not installed (install in venv) | Target >=0.2.8 | MPS fallback |
| chatterbox-tts | PyTorch ChatterBox path | Not installed (install in venv) | Target 0.1.7 | -- |
| torch | MPS/CUDA fallback | In venv (for Qwen) | 2.6.0 | -- |

**Missing dependencies with no fallback:**
- None blocking. ChatterBox deps are installed via `chatter model download --engine chatterbox`.

**Missing dependencies with fallback:**
- `mlx-audio` not in venv yet -- installed as part of this phase's dependency setup. MPS fallback if MLX path fails.

## Sources

### Primary (HIGH confidence)
- [Resemble AI ChatterBox GitHub](https://github.com/resemble-ai/chatterbox) -- Official API: import paths, generate() signatures, model variants, pyproject.toml dependencies
- [chatterbox-tts 0.1.7 on PyPI](https://pypi.org/project/chatterbox-tts/) -- Package version, Python >=3.10 requirement
- [mlx-community/chatterbox-fp16](https://huggingface.co/mlx-community/chatterbox-fp16) -- MLX model, generate API with ref_audio
- [mlx-community/chatterbox-turbo-fp16](https://huggingface.co/mlx-community/chatterbox-turbo-fp16) -- MLX Turbo model, 2.99GB, paralinguistic tags

### Secondary (MEDIUM confidence)
- [Jimmi42/chatterbox-tts-apple-silicon-code](https://huggingface.co/Jimmi42/chatterbox-tts-apple-silicon-code) -- MPS safe loading pattern (CPU first, then .to(device))
- [Chatterbox Apple Silicon install script (issue #336)](https://github.com/resemble-ai/chatterbox/issues/336) -- Curated dependency versions for Apple Silicon
- [Blaizzy/mlx-audio GitHub](https://github.com/Blaizzy/mlx-audio) -- MLX audio library API, ChatterBox support
- [Chatterbox pyproject.toml](https://github.com/resemble-ai/chatterbox/blob/master/pyproject.toml) -- Full dependency list with version pins

### Tertiary (LOW confidence)
- MLX multilingual model availability -- not found, assumed unavailable
- ChatterBox + transformers 4.57.3 compatibility -- untested, needs validation
- spacy-pkuseg on Python 3.12 -- compatibility unknown

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH -- ChatterBox API confirmed via official GitHub, PyPI, HuggingFace model cards
- Architecture: MEDIUM-HIGH -- Follows proven qwen.py pattern. MLX path API verified from model cards. MPS workarounds from community sources.
- Pitfalls: HIGH -- Dependency conflicts confirmed via pyproject.toml. MPS issues confirmed via community forks. OOM risk calculated from model sizes.
- Dependency strategy: MEDIUM -- `--no-deps` pattern verified from community servers. Curated list derived from pyproject.toml. Python 3.12 compat uncertain.

**Research date:** 2026-03-29
**Valid until:** 2026-04-15 (fast-moving: MLX community models may add multilingual support, ChatterBox may update deps)
