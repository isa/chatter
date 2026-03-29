"""
chatter_bridge -- Python adapter normalizing TTS engine APIs.

Dispatcher module: re-exports all public functions and routes calls to the
active engine module. Default engine is "qwen" (Qwen3-TTS via qwen-tts / mlx-audio).

Called from Rust via PyO3. All functions return Python lists (not numpy arrays)
and ensure data is on CPU before returning to Rust.
"""
import sys
import os
import contextlib
import importlib
import warnings

# ---------------------------------------------------------------------------
# Common utilities (shared across engines)
# ---------------------------------------------------------------------------

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


# ---------------------------------------------------------------------------
# Engine dispatch
# ---------------------------------------------------------------------------

_active_engine = None
_active_engine_name = None


def _get_engine():
    """Return the active engine module, loading qwen by default."""
    global _active_engine, _active_engine_name
    if _active_engine is None:
        set_engine("qwen")
    return _active_engine


def set_engine(name):
    """Switch the active engine module.

    Args:
        name: Engine name ("qwen" or "chatterbox").
    """
    global _active_engine, _active_engine_name
    from chatter_bridge.engines import AVAILABLE_ENGINES
    if name not in AVAILABLE_ENGINES:
        raise ValueError(f"Unknown engine: {name!r}. Available: {list(AVAILABLE_ENGINES.keys())}")
    _active_engine = importlib.import_module(AVAILABLE_ENGINES[name])
    _active_engine_name = name


# ---------------------------------------------------------------------------
# Public API -- delegates to the active engine
# ---------------------------------------------------------------------------

def detect_backend():
    """Return 'mlx', 'cuda', 'mps', or 'cpu'."""
    return _get_engine().detect_backend()


def set_mlx_quantization(suffix):
    """Set the MLX quantization suffix (e.g. 'bf16' or '8bit')."""
    return _get_engine().set_mlx_quantization(suffix)


def load_design_model():
    """Load the VoiceDesign model. Caches in engine's _models dict."""
    return _get_engine().load_design_model()


def load_base_model():
    """Load the Base model for clone prompt creation."""
    return _get_engine().load_base_model()


def load_custom_voice_model():
    """Load the CustomVoice model for generation with saved profiles."""
    return _get_engine().load_custom_voice_model()


def voice_design(text, language, instruct):
    """Run VoiceDesign inference. Returns (list_of_floats, sample_rate)."""
    return _get_engine().voice_design(text, language, instruct)


def create_clone_prompt(ref_audio_path, ref_text):
    """Create a reusable voice clone prompt from reference audio."""
    return _get_engine().create_clone_prompt(ref_audio_path, ref_text)


def save_clone_prompt(prompt, path):
    """Save a clone prompt to disk via torch.save."""
    return _get_engine().save_clone_prompt(prompt, path)


def load_clone_prompt(path):
    """Load a clone prompt from disk via torch.load."""
    return _get_engine().load_clone_prompt(path)


def generate_speech(text, language, profile_dir, ref_text="", temperature=0.7, repetition_penalty=1.2):
    """Generate speech from text using a saved profile."""
    return _get_engine().generate_speech(text, language, profile_dir, ref_text, temperature, repetition_penalty)


def voice_clone_from_audio(ref_audio_path, text, language):
    """Generate speech by cloning from a reference audio file directly."""
    return _get_engine().voice_clone_from_audio(ref_audio_path, text, language)


def is_model_loaded(model_type):
    """Check if a model is already cached. Returns True/False."""
    return _get_engine().is_model_loaded(model_type)


def ensure_model(model_type):
    """Load a model by type name if not already cached."""
    return _get_engine().ensure_model(model_type)


def unload_all_models():
    """Release all cached models to free memory."""
    return _get_engine().unload_all_models()
