# Compute Backend Detection Research

**Confidence:** HIGH (PyTorch and MLX stable APIs)

## MPS Detection (Apple Silicon)

```python
torch.backends.mps.is_built()       # compiled with MPS support (always True for PyPI wheels)
torch.backends.mps.is_available()    # actually usable on this system (True on macOS 12.3+ with Metal GPU)
```

Requires PyTorch 1.12+, but qwen-tts installs 2.x anyway.

MPS has **no VRAM figure** — Apple Silicon uses unified memory:
- `sysctl -n hw.memsize` — total RAM in bytes
- `sysctl -n machdep.cpu.brand_string` — chip name (e.g. "Apple M3 Pro")

## CUDA Detection

```python
torch.cuda.is_available()                          # availability check
torch.cuda.get_device_name(0)                      # GPU name string
torch.cuda.get_device_properties(0).total_memory   # VRAM in bytes (/ 1024**3 for GB)
```

## MLX Detection

```python
import mlx.core as mx
mx.metal.is_available()     # MLX Metal check
mx.metal.device_info()      # dict with "memory_size", "architecture", "max_buffer_length"
```

`memory_size` is the Metal GPU's memory budget — distinct from total system RAM. This is the right number for estimating if a model fits on GPU.

## Backend Priority (D-11)

`CUDA > MLX > MPS > CPU (refuse with error)`

MLX preferred over MPS on Mac because mlx-audio uses ~4x less memory and runs faster.

## Doctor Command Data

Use `importlib.metadata.version("package-name")` for version checks — avoids importing heavy packages just to read a version string.

Use `shutil.disk_usage()` for disk space check.

Single `get_system_info()` Python function returning a flat primitive dict — one PyO3 call to gather all facts:
- Python version
- torch version
- qwen-tts version
- Available backends (CUDA, MLX, MPS)
- GPU/accelerator name
- VRAM/memory
- Disk space for models

## Error Message Architecture

- Short one-liners by default (D-05): `error: qwen-tts not installed`
- `--verbose` adds the fix: `run: python3.12 -m pip install -U qwen-tts`
- CPU fallback should be a hard error (inference would take hours)

## Memory Requirements (MEDIUM confidence — community benchmarks)

| Model | CUDA VRAM | MPS RAM | MLX GPU budget |
|-------|-----------|---------|----------------|
| 0.6B  | 2-4 GB    | 8 GB    | 2-4 GB         |
| 1.7B  | 4-8 GB    | 16 GB   | 4-8 GB         |
