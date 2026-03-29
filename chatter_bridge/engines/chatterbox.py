"""
chatter_bridge.engines.chatterbox -- ChatterBox TTS engine implementation.

Provides voice cloning and speech generation using ChatterBox (Resemble AI).
Supports three variants: Original, Turbo, and Multilingual.
Backend detection follows MLX-first on Apple Silicon (except Multilingual,
which has no MLX community model), then CUDA, MPS, CPU fallback.
"""
import os
import gc
import numpy as np

from chatter_bridge import _suppress_output, _suppress_warnings_only

# ---------------------------------------------------------------------------
# Module-level state
# ---------------------------------------------------------------------------

_backend_cache = None
_models = {}
_variant = "original"  # Current variant: "original", "turbo", or "multilingual"

# Language mapping for Multilingual variant
_LANGUAGE_MAP = {
    "English": "en",
    "Chinese": "zh",
    "French": "fr",
    "German": "de",
    "Spanish": "es",
    "Japanese": "ja",
    "Korean": "ko",
    "Portuguese": "pt",
    "Russian": "ru",
    "Italian": "it",
    "auto": "en",
}


# ---------------------------------------------------------------------------
# Private helpers
# ---------------------------------------------------------------------------

def _check_deps():
    """Verify ChatterBox is installed. Raises ImportError with install instructions."""
    try:
        import chatterbox  # noqa: F401
    except ImportError:
        raise ImportError(
            "ChatterBox is not installed. Run:\n"
            "  chatter model download --engine chatterbox\n"
            "to install ChatterBox models and dependencies."
        )


def _mlx_model_id():
    """Return the MLX community model ID for the current variant."""
    if _variant == "original":
        return "mlx-community/chatterbox-fp16"
    elif _variant == "turbo":
        return "mlx-community/chatterbox-turbo-fp16"
    else:
        raise ValueError(
            "No MLX model available for the 'multilingual' variant. "
            "Multilingual falls back to PyTorch (MPS/CUDA/CPU)."
        )


def _load_pytorch_model(variant, device):
    """Load a ChatterBox PyTorch model with MPS-safe pattern.

    Always loads to CPU first, then selectively moves submodels to the
    target device. This avoids the MPS operator-not-supported crash
    (Pitfall 1 from research).
    """
    with _suppress_warnings_only():
        if variant == "turbo":
            from chatterbox.tts_turbo import ChatterboxTurboTTS
            model = ChatterboxTurboTTS.from_pretrained("cpu")
        elif variant == "multilingual":
            from chatterbox.mtl_tts import ChatterboxMultilingualTTS
            model = ChatterboxMultilingualTTS.from_pretrained("cpu")
        else:
            from chatterbox.tts import ChatterboxTTS
            model = ChatterboxTTS.from_pretrained("cpu")

    if device != "cpu":
        model.t3 = model.t3.to(device)
        model.s3gen = model.s3gen.to(device)
        model.ve = model.ve.to(device)

    return model


# ---------------------------------------------------------------------------
# Public API -- engine interface contract
# ---------------------------------------------------------------------------

def detect_backend():
    """Return 'mlx', 'cuda', 'mps', or 'cpu'.

    MLX is preferred on Apple Silicon for Original and Turbo variants.
    Multilingual variant skips MLX detection (no community model exists)
    and falls back to MPS/CUDA/CPU.
    """
    global _backend_cache
    _check_deps()

    if _backend_cache is not None:
        return _backend_cache

    # MLX only available for non-multilingual variants (Pitfall 4)
    if _variant != "multilingual":
        try:
            import mlx.core as mx
            if mx.metal.is_available():
                _backend_cache = "mlx"
                return "mlx"
        except ImportError:
            pass

    try:
        import torch
        if torch.cuda.is_available():
            _backend_cache = "cuda"
            return "cuda"
        if hasattr(torch.backends, "mps") and torch.backends.mps.is_available():
            _backend_cache = "mps"
            return "mps"
    except ImportError:
        pass

    _backend_cache = "cpu"
    return "cpu"


def set_mlx_quantization(suffix):
    """No-op for ChatterBox (no quantization variants available)."""
    pass


def set_variant(variant_str):
    """Set the ChatterBox model variant.

    Args:
        variant_str: One of "original", "turbo", "multilingual".

    If the variant changes, cached models are unloaded (model class differs
    per variant) and the backend cache is reset (MLX availability depends
    on variant -- multilingual has no MLX model).
    """
    global _variant, _backend_cache
    valid = ("original", "turbo", "multilingual")
    if variant_str not in valid:
        raise ValueError(f"Unknown ChatterBox variant: {variant_str!r}. Must be one of {valid}")

    if variant_str != _variant:
        _variant = variant_str
        _backend_cache = None  # Backend may change (MLX vs MPS for multilingual)
        unload_all_models()


def load_design_model():
    """ChatterBox does not support voice design."""
    raise NotImplementedError(
        "ChatterBox does not support voice design. Use --engine qwen for voice design."
    )


def load_base_model():
    """Load the ChatterBox model for the current variant. Caches in _models dict."""
    if "base" in _models:
        return _models["base"]

    _check_deps()
    backend = detect_backend()

    with _suppress_output():
        if backend == "mlx":
            from mlx_audio.tts.utils import load_model
            model = load_model(_mlx_model_id())
        else:
            device = "cuda:0" if backend == "cuda" else "mps" if backend == "mps" else "cpu"
            model = _load_pytorch_model(_variant, device)

    _models["base"] = model
    return model


def load_custom_voice_model():
    """Alias to load_base_model -- ChatterBox uses the same model for cloning and generation."""
    return load_base_model()


def voice_design(text, language, instruct):
    """ChatterBox does not support voice design."""
    raise NotImplementedError(
        "ChatterBox does not support voice design. Use --engine qwen for voice design."
    )


def create_clone_prompt(ref_audio_path, ref_text):
    """No-op -- ChatterBox uses ref_audio directly, no pre-computed prompts (D-04)."""
    return None


def save_clone_prompt(prompt, path):
    """No-op -- ChatterBox does not use pre-computed clone prompts."""
    pass


def load_clone_prompt(path):
    """No-op -- ChatterBox does not use pre-computed clone prompts."""
    return None


def _generate_with_model(model, backend, text, language, ref_audio_path, exaggeration=0.5, cfg_weight=0.5):
    """Shared generation logic for both generate_speech and voice_clone_from_audio.

    Returns (list_of_floats, sample_rate).
    """
    with _suppress_output():
        if backend == "mlx":
            results = list(model.generate(text=text, ref_audio=ref_audio_path))
            audio = np.array(results[0].audio, dtype=np.float32)
            return audio.tolist(), 24000
        else:
            # PyTorch path -- variant-aware generation
            if _variant == "multilingual":
                lang_id = _LANGUAGE_MAP.get(language, "en")
                wav = model.generate(
                    text=text,
                    audio_prompt_path=ref_audio_path,
                    language_id=lang_id,
                )
            elif _variant == "original":
                wav = model.generate(
                    text=text,
                    audio_prompt_path=ref_audio_path,
                    exaggeration=exaggeration,
                    cfg_weight=cfg_weight,
                )
            else:
                # Turbo variant
                wav = model.generate(
                    text=text,
                    audio_prompt_path=ref_audio_path,
                )
            audio = wav.squeeze().cpu().numpy().astype(np.float32)
            return audio.tolist(), 24000


def generate_speech(text, language, profile_dir, ref_text="", temperature=0.7, repetition_penalty=1.2, exaggeration=0.5, cfg_weight=0.5):
    """Generate speech from text using a saved ChatterBox voice profile.

    The profile_dir must contain ref_audio.wav (the reference audio for voice cloning).
    temperature and repetition_penalty are accepted for API compatibility but not
    used by ChatterBox (its generation params are variant-specific).
    exaggeration and cfg_weight are used by ChatterBox Original variant only.

    Returns (list_of_floats, sample_rate).
    """
    _check_deps()
    backend = detect_backend()
    ref_audio_path = os.path.join(profile_dir, "ref_audio.wav")

    if not os.path.exists(ref_audio_path):
        raise FileNotFoundError(f"Reference audio not found: {ref_audio_path}")

    model = load_base_model()
    return _generate_with_model(model, backend, text, language, ref_audio_path, exaggeration=exaggeration, cfg_weight=cfg_weight)


def voice_clone_from_audio(ref_audio_path, text, language):
    """Generate speech by cloning from a reference audio file directly.

    Used during clone profile creation to generate the preview sample.
    Takes ref_audio_path directly instead of reading from profile_dir.

    Returns (list_of_floats, sample_rate).
    """
    _check_deps()
    backend = detect_backend()
    model = load_base_model()
    return _generate_with_model(model, backend, text, language, ref_audio_path)


def is_model_loaded(model_type):
    """Check if a model is already cached. Returns True/False."""
    return model_type in _models


def ensure_model(model_type):
    """Load a model if not already cached.

    All ChatterBox model types map to the "base" model (same model does
    cloning and generation).

    Returns True if the model was already loaded, False if freshly loaded.
    """
    _check_deps()
    was_loaded = "base" in _models
    load_base_model()
    return was_loaded


def unload_all_models():
    """Release all cached models to free GPU/system memory.

    Per D-09: full cleanup including gc.collect() and device cache clearing.
    """
    global _models, _backend_cache
    for key in list(_models.keys()):
        del _models[key]
    _models.clear()

    gc.collect()

    try:
        import torch
        if torch.cuda.is_available():
            torch.cuda.empty_cache()
        if hasattr(torch.backends, "mps") and torch.backends.mps.is_available():
            torch.mps.empty_cache()
    except ImportError:
        pass  # MLX-only path has no torch

    _backend_cache = None
