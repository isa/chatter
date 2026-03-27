# Qwen3-TTS API and mlx-audio Research Findings

**Confidence:** MEDIUM-HIGH (synthesized from existing project research documents and training knowledge; live verification via WebFetch/WebSearch/Bash was unavailable)

## 1. qwen-tts 0.1.1 API

### Core Class: `Qwen3TTSModel`

The main entry point is `qwen_tts.Qwen3TTSModel`. It wraps the HuggingFace transformers model with TTS-specific methods.

**Loading a model:**

```python
from qwen_tts import Qwen3TTSModel
import torch

model = Qwen3TTSModel.from_pretrained(
    model_name_or_path,           # HuggingFace model ID or local path
    device_map="cuda:0",          # "cuda:0", "mps", "cpu", "auto"
    dtype=torch.bfloat16,         # bfloat16 (CUDA), float16 (MPS), float32 (MPS+clone)
    attn_implementation="flash_attention_2",  # CUDA only, optional
)
```

**Three distinct generation methods, each requiring a different model variant:**

1. **`generate_voice_design(text, language, instruct)`** -- VoiceDesign model. Creates a voice from a natural language description. Non-deterministic.
2. **`generate_voice_clone(text, language, voice_clone_prompt)`** -- Base model. Uses a prompt created by `create_voice_clone_prompt(ref_audio, ref_text)`.
3. **`generate_custom_voice(text, speaker, language, instruct)`** -- CustomVoice model. Uses pre-built named speakers.

All return an iterable of result objects with `.audio` (numpy array) and `.sr` (sample rate, typically 24000 Hz).

## 2. Model Variants (Complete Registry)

| Model ID | Size | Use Case |
|----------|------|----------|
| `Qwen/Qwen3-TTS-12Hz-1.7B-VoiceDesign` | 1.7B | Voice design from text descriptions |
| `Qwen/Qwen3-TTS-12Hz-1.7B-CustomVoice` | 1.7B | Named voice TTS |
| `Qwen/Qwen3-TTS-12Hz-0.6B-CustomVoice` | 0.6B | Named voice TTS (smaller) |
| `Qwen/Qwen3-TTS-12Hz-1.7B-Base` | 1.7B | Voice cloning |
| `Qwen/Qwen3-TTS-12Hz-0.6B-Base` | 0.6B | Voice cloning (smaller) |

VoiceDesign only exists as 1.7B. The 0.6B model has significant quality degradation on long text (106 long pauses vs 2 for 1.7B).

Approximate resource requirements: 0.6B variants need ~1.2 GB download / ~2 GB VRAM. 1.7B variants need ~3.4 GB download / ~6 GB VRAM (LOW-MEDIUM confidence on exact numbers).

## 3. Device/Backend Support

**CUDA:** `device_map="cuda:0"`, `dtype=torch.bfloat16`, optional `attn_implementation="flash_attention_2"`

**MPS (Apple Silicon):** `device_map="mps"`, `dtype=torch.float16` for CustomVoice/VoiceDesign. **CRITICAL:** Base (clone) models MUST use `torch.float32` on MPS -- float16 causes NaN errors. Do NOT pass `attn_implementation` on MPS.

**CPU:** `device_map="cpu"`, `dtype=torch.float32`. Extremely slow, test-only.

**Runtime detection:**
```python
torch.cuda.is_available()          # CUDA
torch.backends.mps.is_available()  # MPS
```
For MLX: `import mlx.core as mx` succeeds.

## 4. mlx-audio Alternative

**Yes, mlx-audio supports Qwen3-TTS.** API is different:

```python
from mlx_audio.tts.utils import load_model

model = load_model("mlx-community/Qwen3-TTS-12Hz-1.7B-CustomVoice-bf16")
results = list(model.generate_custom_voice(
    text="Hello world", speaker="Ryan",
    language="English", instruct="Speak clearly",
))
audio = results[0].audio   # mx.array (MLX, not numpy)
```

**Key differences from qwen-tts:**
- Apple Silicon only (MLX framework)
- Different model format (MLX-converted weights from `mlx-community/` HF org)
- Reports ~2-3 GB RAM vs ~6 GB for qwen-tts (LOW confidence -- community claims)
- Returns `mx.array` not numpy
- **Unverified:** whether it supports voice cloning (`generate_voice_clone`, `create_voice_clone_prompt`) and voice design (`generate_voice_design`). Only `generate_custom_voice` is confirmed.

Only `mlx-community/Qwen3-TTS-12Hz-1.7B-CustomVoice-bf16` is a verified model ID. Other variants are inferred from naming patterns.

## 5. Model Download/Caching

**Default HF cache:** `~/.cache/huggingface/hub/models--Qwen--Qwen3-TTS-12Hz-{variant}/`

**Control cache location:** Set `HF_HOME` or `HF_HUB_CACHE` env vars, or pass `cache_dir=` to `from_pretrained`.

**Check if cached:** Check if `~/.cache/huggingface/hub/models--Qwen--Qwen3-TTS-12Hz-{variant}` directory exists, or use `huggingface_hub.scan_cache_dir()` for a programmatic listing.

**Download without loading:** Use `huggingface_hub.snapshot_download(repo_id="Qwen/...")` for `chatter model download`. This downloads all model files without loading into memory.

**Recommendation:** Use HF default cache (don't move models). Document location in `chatter doctor`.

## 6. Supported Languages

10 languages + auto-detect: Chinese, English, Japanese, Korean, French, German, Spanish, Portuguese, Russian, Italian. Pass `"Auto"` for auto-detection.

## 7. Critical Open Questions (Need Hands-On Validation)

1. **Exact generate method return type** -- generator vs list? `.audio`/`.sr` attributes vs tuple?
2. **mlx-audio voice cloning/design support** -- if missing, Mac clone/design must use qwen-tts+MPS
3. **Python 3.12 vs 3.10 conflict** -- PITFALLS.md says 3.10.x only; STACK.md says 3.12. Must test `pip install qwen-tts` in Python 3.12 venv
4. **Progress callback from generate** -- does qwen-tts accept a callback parameter, or spinner-only?
5. **Sample rate** -- always 24000 Hz across all variants?
