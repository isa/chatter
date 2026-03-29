"""
chatter_bridge.engines.chatterbox -- ChatterBox TTS engine (stub).

All functions raise NotImplementedError until the engine is implemented.
"""


_backend_cache = None
_models = {}


def detect_backend():
    raise NotImplementedError("ChatterBox engine not yet implemented")


def set_mlx_quantization(suffix):
    raise NotImplementedError("ChatterBox engine not yet implemented")


def load_design_model():
    raise NotImplementedError("ChatterBox engine not yet implemented")


def load_base_model():
    raise NotImplementedError("ChatterBox engine not yet implemented")


def load_custom_voice_model():
    raise NotImplementedError("ChatterBox engine not yet implemented")


def voice_design(text, language, instruct):
    raise NotImplementedError("ChatterBox engine not yet implemented")


def create_clone_prompt(ref_audio_path, ref_text):
    raise NotImplementedError("ChatterBox engine not yet implemented")


def save_clone_prompt(prompt, path):
    raise NotImplementedError("ChatterBox engine not yet implemented")


def load_clone_prompt(path):
    raise NotImplementedError("ChatterBox engine not yet implemented")


def generate_speech(text, language, profile_dir, ref_text="", temperature=0.7, repetition_penalty=1.2, exaggeration=0.5, cfg_weight=0.5):
    raise NotImplementedError("ChatterBox engine not yet implemented")


def voice_clone_from_audio(ref_audio_path, text, language):
    raise NotImplementedError("ChatterBox engine not yet implemented")


def is_model_loaded(model_type):
    raise NotImplementedError("ChatterBox engine not yet implemented")


def ensure_model(model_type):
    raise NotImplementedError("ChatterBox engine not yet implemented")


def unload_all_models():
    raise NotImplementedError("ChatterBox engine not yet implemented")
