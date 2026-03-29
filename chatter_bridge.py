"""
chatter_bridge.py -- Python adapter normalizing qwen-tts and mlx-audio APIs.

Called from Rust via PyO3. All functions return Python lists (not numpy arrays)
and ensure data is on CPU before returning to Rust.
"""
import sys
import os
import contextlib
import warnings

_backend_cache = None
_models = {}


@contextlib.contextmanager
def _suppress_output():
    """Suppress stdout/stderr from noisy Python libraries during inference.

    Captures all output to devnull so it doesn't fight with Rust's indicatif spinner.
    Only used around inference calls -- model loading lets output through so users
    see download progress and checkpoint loading status.
    """
    with warnings.catch_warnings():
        warnings.simplefilter("ignore")
        old_stdout = sys.stdout
        old_stderr = sys.stderr
        try:
            sys.stdout = open(os.devnull, "w")
            sys.stderr = open(os.devnull, "w")
            yield
        finally:
            sys.stdout.close()
            sys.stderr.close()
            sys.stdout = old_stdout
            sys.stderr = old_stderr


@contextlib.contextmanager
def _suppress_warnings_only():
    """Suppress Python warnings but let stdout/stderr through.

    Used during model loading so HF download progress and checkpoint loading
    messages are visible, while noisy warnings (flash-attn, deprecation) are hidden.
    """
    with warnings.catch_warnings():
        warnings.simplefilter("ignore")
        # Suppress only stdout (import chatter, library banners) but keep stderr
        # so that tqdm/HF progress bars (which write to stderr) are visible.
        old_stdout = sys.stdout
        try:
            sys.stdout = open(os.devnull, "w")
            yield
        finally:
            sys.stdout.close()
            sys.stdout = old_stdout


def detect_backend():
    """Return 'mlx', 'cuda', 'mps', or 'cpu'."""
    global _backend_cache
    if _backend_cache is not None:
        return _backend_cache
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


def load_design_model(quantization="bf16"):
    """Load the VoiceDesign model. Caches in _models dict keyed by quantization."""
    cache_key = f"design_{quantization}"
    if cache_key in _models:
        return _models[cache_key]
    backend = detect_backend()
    with _suppress_output():
        if backend == "mlx":
            from mlx_audio.tts.utils import load_model
            model = load_model(f"mlx-community/Qwen3-TTS-12Hz-1.7B-VoiceDesign-{quantization}")
        else:
            from qwen_tts import Qwen3TTSModel
            import torch
            device = "cuda:0" if backend == "cuda" else "mps" if backend == "mps" else "cpu"
            dtype = torch.bfloat16 if backend == "cuda" else torch.float32
            model = Qwen3TTSModel.from_pretrained(
                "Qwen/Qwen3-TTS-12Hz-1.7B-VoiceDesign",
                device_map=device, dtype=dtype,
            )
    _models[cache_key] = model
    return model


def load_base_model(quantization="bf16"):
    """Load the Base model for clone prompt creation. Caches in _models dict keyed by quantization."""
    cache_key = f"base_{quantization}"
    if cache_key in _models:
        return _models[cache_key]
    backend = detect_backend()
    with _suppress_output():
        if backend == "mlx":
            from mlx_audio.tts.utils import load_model
            model = load_model(f"mlx-community/Qwen3-TTS-12Hz-1.7B-Base-{quantization}")
        else:
            from qwen_tts import Qwen3TTSModel
            import torch
            device = "cuda:0" if backend == "cuda" else "mps" if backend == "mps" else "cpu"
            dtype = torch.bfloat16 if backend == "cuda" else torch.float32
            model = Qwen3TTSModel.from_pretrained(
                "Qwen/Qwen3-TTS-12Hz-1.7B-Base",
                device_map=device, dtype=dtype,
            )
    _models[cache_key] = model
    return model


def load_custom_voice_model(quantization="bf16"):
    """Load the CustomVoice model for generation with saved profiles. Caches in _models dict keyed by quantization."""
    cache_key = f"custom_{quantization}"
    if cache_key in _models:
        return _models[cache_key]
    backend = detect_backend()
    with _suppress_output():
        if backend == "mlx":
            from mlx_audio.tts.utils import load_model
            model = load_model(f"mlx-community/Qwen3-TTS-12Hz-1.7B-CustomVoice-{quantization}")
        else:
            from qwen_tts import Qwen3TTSModel
            import torch
            device = "cuda:0" if backend == "cuda" else "mps" if backend == "mps" else "cpu"
            dtype = torch.bfloat16 if backend == "cuda" else torch.float32
            model = Qwen3TTSModel.from_pretrained(
                "Qwen/Qwen3-TTS-12Hz-1.7B-CustomVoice",
                device_map=device, dtype=dtype,
            )
    _models[cache_key] = model
    return model


def voice_design(text, language, instruct, quantization="bf16"):
    """Run VoiceDesign inference. Returns (list_of_floats, sample_rate)."""
    import numpy as np
    model = load_design_model(quantization=quantization)
    backend = detect_backend()
    with _suppress_output():
        if backend == "mlx":
            results = list(model.generate_voice_design(
                text=text, language=language, instruct=instruct,
                temperature=0.5
            ))
            audio = np.array(results[0].audio, dtype=np.float32)
            return audio.tolist(), 24000
        else:
            wavs, sr = model.generate_voice_design(
                text=text, language=language, instruct=instruct,
                temperature=0.5
            )
            audio = wavs[0].cpu().numpy().astype(np.float32)
            return audio.tolist(), int(sr)


def create_clone_prompt(ref_audio_path, ref_text, quantization="bf16"):
    """Create a reusable voice clone prompt from reference audio.
    Returns the prompt object (opaque -- save with save_clone_prompt).
    For MLX, returns None (MLX uses ref_audio directly).
    """
    backend = detect_backend()
    if backend == "mlx":
        return None  # MLX doesn't have clone prompts; uses ref_audio directly
    model = load_base_model(quantization=quantization)
    with _suppress_output():
        import soundfile as sf
        wav, sr = sf.read(ref_audio_path)
        prompt = model.create_voice_clone_prompt(
            ref_audio=(wav, sr), ref_text=ref_text
        )
    return prompt


def save_clone_prompt(prompt, path):
    """Save a clone prompt to disk via torch.save."""
    import torch
    torch.save(prompt, path)


def load_clone_prompt(path):
    """Load a clone prompt from disk via torch.load."""
    import torch
    backend = detect_backend()
    device = "cuda:0" if backend == "cuda" else "mps" if backend == "mps" else "cpu"
    return torch.load(path, map_location=device)


def generate_speech(text, language, profile_dir, ref_text="", temperature=0.7, repetition_penalty=1.2, quantization="bf16"):
    """Generate speech from text using a saved profile.
    profile_dir should contain either voice_prompt.bin (CUDA/MPS) or ref_audio.wav (MLX).
    ref_text is the transcript of ref_audio.wav (needed for MLX voice cloning).
    temperature and repetition_penalty control pacing (lower temp = more natural pauses).
    Returns (list_of_floats, sample_rate).
    """
    import numpy as np
    backend = detect_backend()
    with _suppress_output():
        if backend == "mlx":
            # MLX voice cloning uses the Base model with ref_audio + ref_text
            model = load_base_model(quantization=quantization)
            ref_audio_path = os.path.join(profile_dir, "ref_audio.wav")
            results = list(model.generate(
                text=text, language=language,
                ref_audio=ref_audio_path, ref_text=ref_text,
                temperature=temperature, repetition_penalty=repetition_penalty,
            ))
            if not results or not hasattr(results[0], 'audio'):
                raise ValueError(
                    f"Model returned no audio for text ({len(text)} chars). "
                    "Text may be too long for a single chunk."
                )
            audio = np.array(results[0].audio, dtype=np.float32)
            return audio.tolist(), 24000
        else:
            model = load_custom_voice_model(quantization=quantization)
            import torch
            prompt_path = os.path.join(profile_dir, "voice_prompt.bin")
            device = "cuda:0" if backend == "cuda" else "mps" if backend == "mps" else "cpu"
            prompt = torch.load(prompt_path, map_location=device)
            wavs, sr = model.generate_voice_clone(
                text=text, language=language, voice_clone_prompt=prompt,
                temperature=temperature, repetition_penalty=repetition_penalty,
            )
            audio = wavs[0].cpu().numpy().astype(np.float32)
            return audio.tolist(), int(sr)


def voice_clone_from_audio(ref_audio_path, text, language, quantization="bf16"):
    """Generate speech by cloning from a reference audio file directly.
    Used during clone profile creation to generate the preview sample.
    Returns (list_of_floats, sample_rate).
    """
    import numpy as np
    backend = detect_backend()
    # MLX voice cloning uses Base model with ref_audio (ICL), not CustomVoice
    if backend == "mlx":
        model = load_base_model(quantization=quantization)
    else:
        model = load_custom_voice_model(quantization=quantization)
    with _suppress_output():
        if backend == "mlx":
            results = list(model.generate(
                text=text, language=language, ref_audio=ref_audio_path
            ))
            audio = np.array(results[0].audio, dtype=np.float32)
            return audio.tolist(), 24000
        else:
            import soundfile as sf
            wav_data, sr_data = sf.read(ref_audio_path)
            prompt = model.create_voice_clone_prompt(
                ref_audio=(wav_data, sr_data), ref_text=text
            )
            wavs, sr = model.generate_voice_clone(
                text=text, language=language, voice_clone_prompt=prompt
            )
            audio = wavs[0].cpu().numpy().astype(np.float32)
            return audio.tolist(), int(sr)


def is_model_loaded(model_type):
    """Check if a model is already cached. Returns True/False."""
    return model_type in _models


def ensure_model(model_type, quantization="bf16"):
    """Load a model by type name if not already cached.
    model_type: 'design', 'base', or 'custom'
    quantization: 'bf16' or '8bit' (MLX only, ignored for PyTorch)
    Returns True if the model was already loaded, False if it had to be loaded now.
    For MLX, 'custom' loads the base model (MLX voice cloning uses Base, not CustomVoice).
    """
    backend = detect_backend()
    # MLX voice cloning uses the Base model, not CustomVoice
    effective_type = model_type
    if model_type == "custom" and backend == "mlx":
        effective_type = "base"
    cache_key = f"{effective_type}_{quantization}"
    was_loaded = cache_key in _models
    if effective_type == "design":
        load_design_model(quantization=quantization)
    elif effective_type == "base":
        load_base_model(quantization=quantization)
    elif effective_type == "custom":
        load_custom_voice_model(quantization=quantization)
    else:
        raise ValueError(f"Unknown model type: {model_type}")
    return was_loaded


def unload_all_models():
    """Release all cached models to free memory."""
    global _models
    _models.clear()
    backend = detect_backend()
    if backend != "mlx":
        import torch
        if torch.cuda.is_available():
            torch.cuda.empty_cache()
    import gc
    gc.collect()
