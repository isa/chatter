"""
chatter_bridge.engines -- Engine registry.

Each engine module must expose the same set of public functions.
"""

AVAILABLE_ENGINES = {
    "qwen": "chatter_bridge.engines.qwen",
    "chatterbox": "chatter_bridge.engines.chatterbox",
}
