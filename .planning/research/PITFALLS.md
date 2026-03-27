# Pitfalls Research

**Domain:** Rust CLI with PyO3 embedding for Qwen3-TTS local inference
**Researched:** 2026-03-27
**Confidence:** HIGH (verified against PyO3 issues, Qwen3-TTS GitHub issues, PyTorch forums)

## Critical Pitfalls

### Pitfall 1: qwen-tts Package Dependency Hell

**What goes wrong:**
The `qwen-tts` pip package (v0.1.1) hardcodes `transformers==4.57.3` in setup.py, but the Qwen3-TTS model architecture actually requires `transformers >= 5.x`. The `check_model_inputs` function does not exist in transformers 4.57.3. Additionally, the dependency chain `qwen-tts -> librosa -> numba -> llvmlite` constrains Python to a narrow version window. `accelerate==1.12.0` requires Python >= 3.10, while `llvmlite==0.36.0` requires Python < 3.11, creating a Python 3.10.x-only viable window.

**Why it happens:**
The qwen-tts package is young and its pinned dependencies lag behind what the model code actually needs. This is a fast-moving space with breaking changes between transformers major versions.

**How to avoid:**
- Pin Python 3.10.x explicitly in project requirements and documentation
- Test with `pip install qwen-tts` in a clean venv before writing any Rust code -- if the package is broken at install time, consider installing from git with overridden dependencies
- Monitor the Qwen3-TTS GitHub issues (especially #237 and #145) for resolution
- Consider maintaining a `requirements.txt` that pins known-working versions and installs with `--no-deps` followed by manual dependency install

**Warning signs:**
- `ImportError` or `ModuleNotFoundError` when importing `qwen_tts` after pip install
- Conflicting version resolution errors during pip install
- Model loading fails with `AttributeError` on transformers functions

**Phase to address:**
Phase 1 (Foundation) -- resolve this before writing any PyO3 integration code. If the Python side does not install cleanly, nothing else works.

---

### Pitfall 2: PyO3 Virtual Environment Detection Fails Silently

**What goes wrong:**
PyO3 at runtime does not reliably find packages installed in a virtual environment. The embedded Python interpreter may use the system Python's site-packages instead of the venv where `qwen-tts` was installed. Users report that PyO3 ignores `PYO3_PYTHON` and `LD_LIBRARY_PATH` pointing to venvs. The binary compiles fine but fails at runtime with `ModuleNotFoundError: No module named 'qwen_tts'`.

**Why it happens:**
PyO3 auto-initialize starts a Python interpreter, but the interpreter's `sys.path` is determined at compile time based on the Python installation PyO3 linked against. If the user's venv is different from the build-time Python, or if environment variables are not propagated correctly, the interpreter will not see venv packages.

**How to avoid:**
- At startup, explicitly configure `sys.path` via PyO3 to include the correct site-packages directory
- Provide a `chatter setup` command that creates and validates a managed venv at `~/.config/chatter/venv/`
- At runtime, detect VIRTUAL_ENV environment variable and inject its site-packages into `sys.path` before any imports
- Set `PYTHONPATH` and `PYTHONHOME` via `std::env::set_var` before the first `Python::with_gil` call

**Warning signs:**
- Works on developer machine, fails on user machine
- `ModuleNotFoundError` for packages that are clearly installed
- Different behavior when running from shell vs. from a launcher/PATH

**Phase to address:**
Phase 1 (Foundation) -- the Python environment bootstrapping must be rock-solid before any model code runs.

---

### Pitfall 3: GIL Deadlocks in Async/Threaded Contexts

**What goes wrong:**
Calling `Python::with_gil` from a thread that already holds the GIL, or from within a Rust mutex lock, causes deadlock. The program hangs permanently with no error message. This is especially dangerous when trying to run inference on a background thread while updating a progress bar on the main thread.

**Why it happens:**
The Python GIL is re-entrant on the same thread but blocks on other threads. If you hold a Rust `Mutex` and then try to acquire the GIL on another thread (which is also waiting for the mutex), you get a classic lock-ordering deadlock. This commonly happens with `lazy_static`, `once_cell`, or async runtimes like tokio.

**How to avoid:**
- Never hold a Rust `Mutex` while calling `Python::with_gil` -- release the GIL first with `py.allow_threads()`
- Use `GILOnceCell` (PyO3 built-in) instead of `lazy_static` or `OnceLock` for Python objects
- For progress callbacks: release the GIL during inference (`py.allow_threads`), let Python's callback mechanism handle progress, and collect results after
- Keep Python interaction on a single dedicated thread; communicate via channels

**Warning signs:**
- Application hangs during inference with no error output
- Hangs are intermittent and depend on timing
- Hangs only occur in release builds or under load (not in simple tests)

**Phase to address:**
Phase 2 (Core TTS) -- when implementing inference with progress feedback. Design the threading model upfront.

---

### Pitfall 4: GPU Memory Not Released Between Inference Calls

**What goes wrong:**
PyTorch CUDA tensors created during inference are not garbage collected between calls when invoked through PyO3. GPU memory accumulates across multiple TTS generations until CUDA OOM crashes the process. This is particularly severe because the 1.7B model already uses ~6 GB VRAM.

**Why it happens:**
PyO3 Python object references (Py<PyAny>) prevent Python's garbage collector from freeing tensors. If Rust holds any reference to a Python object that transitively references CUDA tensors, those tensors stay in GPU memory. Additionally, `torch.no_grad()` has a documented PyTorch bug where certain batch sizes cause increased memory usage.

**How to avoid:**
- Always wrap inference in `torch.inference_mode()` (not just `torch.no_grad()`)
- After each inference call, explicitly call `torch.cuda.empty_cache()` via PyO3
- Drop all `Py<PyAny>` references to model outputs before starting the next inference
- Scope PyO3 GIL acquisitions tightly -- do not hold Python objects across `with_gil` boundaries
- Use `del` on Python-side tensor variables within the PyO3 call

**Warning signs:**
- Second or third TTS generation crashes with CUDA OOM
- `nvidia-smi` shows increasing memory usage across CLI invocations within same process
- Memory usage grows when processing multi-chapter documents

**Phase to address:**
Phase 2 (Core TTS) and Phase 3 (File Processing) -- critical when the tool processes documents with multiple pages requiring sequential inference calls.

---

### Pitfall 5: Voice Design Outputs Are Not Directly Reusable

**What goes wrong:**
Developers assume VoiceDesign output (the designed voice) can be stored as a simple config and replayed. In reality, VoiceDesign produces a short audio clip, and to reuse that voice you must feed the clip into `create_voice_clone_prompt` to build a reusable prompt, then use `generate_voice_clone` with that prompt. Storing just the text description does not reproduce the same voice.

**Why it happens:**
VoiceDesign is non-deterministic -- the same text description produces different voices each run. The voice identity lives in the generated audio sample, not in the text prompt. The documentation does not make this two-step reuse pattern obvious.

**How to avoid:**
- Voice profile storage must include: (1) the original text description, (2) the generated audio sample WAV, (3) the pre-computed voice clone prompt extracted from that sample
- On profile creation, immediately run `create_voice_clone_prompt` and cache the result
- For voice cloning profiles, store both the reference audio and the clone prompt
- Document that "designing a voice" is a generative process -- users should preview and confirm before saving

**Warning signs:**
- Users report "my voice sounds different every time"
- Voice profiles take unexpectedly long to load (re-extracting features each time)
- Profiles stored as JSON with only text description, no audio

**Phase to address:**
Phase 2 (Voice Profiles) -- the profile storage schema must be designed with this in mind from day one.

---

### Pitfall 6: Long Text Causes Model Hangs or Speed Drift

**What goes wrong:**
For text exceeding ~100 characters, the model's speaking rate gradually accelerates toward the end. For very long text, the model can fail to emit an end-of-sequence token and enter an infinite generation loop, hanging forever. The 0.6B model is significantly worse at this, producing 106 pauses longer than 1.5 seconds versus just 2 for the 1.7B model on the same text.

**Why it happens:**
Autoregressive TTS models accumulate drift over long sequences. The attention mechanism degrades for very long inputs. VoiceDesign mode has a known bug where text splitting is ignored -- only voice cloning mode properly handles long text through chunking.

**How to avoid:**
- Implement paragraph-aware text splitting with sentence-length limits before sending to the model
- Default to the 1.7B model for any text longer than a few sentences
- Set a maximum token generation limit to prevent infinite loops (with timeout fallback)
- For file processing (PDF/TXT), split by paragraphs and concatenate audio output
- Add generation timeout -- if a single chunk takes more than 2x expected time, kill and retry

**Warning signs:**
- Generated audio speeds up noticeably in the last third
- CLI hangs during generation with GPU utilization stuck at 100%
- Audio output is many times longer than expected for the input text length

**Phase to address:**
Phase 3 (File Processing) -- this is where long text is the default. Must have robust chunking before processing documents.

---

### Pitfall 7: First-Run Model Download With No Feedback

**What goes wrong:**
The first time a user runs the tool, Hugging Face transformers silently downloads 3-7 GB of model weights. If there is no progress indication, users think the tool is broken and kill it. Partial downloads corrupt the cache and cause cryptic errors on subsequent runs.

**Why it happens:**
HuggingFace's `from_pretrained` downloads happen inside the Python layer, invisible to the Rust CLI by default. The download progress goes to stderr in Python, which may not be captured or displayed by the Rust process.

**How to avoid:**
- On first run, check if model files exist in `~/.cache/huggingface/hub/` before calling inference
- If missing, display an explicit "Downloading model (X GB)..." message with progress bar
- Intercept HuggingFace download progress callbacks via PyO3 to feed Rust-side progress bars
- Provide a `chatter download` command that pre-downloads models explicitly
- Handle interrupted downloads gracefully: detect partial files and offer to re-download

**Warning signs:**
- Users report "tool hangs on first use"
- Bug reports about corrupted model files
- Inconsistent behavior between first and subsequent runs

**Phase to address:**
Phase 1 (Foundation) -- model download UX must be solved before any public release.

---

## Technical Debt Patterns

| Shortcut | Immediate Benefit | Long-term Cost | When Acceptable |
|----------|-------------------|----------------|-----------------|
| Subprocess instead of PyO3 | Avoids GIL complexity entirely | Loses type safety, error handling, progress callbacks; parsing stdout is fragile | Never for this project -- PyO3 is a core requirement |
| Hardcoded Python path | Works on dev machine | Breaks on every other machine | Never -- must detect at runtime |
| System Python instead of managed venv | Fewer moving parts | Dependency conflicts with user's other Python packages | Only during early prototyping, replace by Phase 1 end |
| Storing voice profiles as audio-only | Simple implementation | Loses metadata, requires re-extraction on every use | Only for MVP if time-pressured, fix in Phase 2 |
| No text chunking for file input | Simpler pipeline | Model hangs/degrades on long text | Never for file processing -- implement from day one |
| Synchronous inference on main thread | Simpler code, no threading | Blocks CLI, no progress bars possible, no Ctrl+C handling | Only for initial prototype |

## Integration Gotchas

| Integration | Common Mistake | Correct Approach |
|-------------|----------------|------------------|
| PyO3 + Python interpreter | Using `auto-initialize` without configuring sys.path | Set PYTHONPATH/PYTHONHOME before first `Python::with_gil`, verify imports work |
| HuggingFace model loading | Assuming models are already cached | Check cache dir first, provide explicit download command, handle network errors |
| qwen-tts package | Installing with plain `pip install qwen-tts` | Pin working dependency versions, test in clean venv, track upstream issue fixes |
| CUDA/GPU detection | Assuming CUDA is available | Check `torch.cuda.is_available()` at startup, provide clear error if no GPU found |
| WAV-to-MP3 encoding | Using FFmpeg subprocess for conversion | Use `mp3lame-encoder` Rust crate for native encoding -- no external dependency |
| Reference audio for cloning | Accepting any audio length/format | Validate 3-15 second range, warn about quality, convert to required format before processing |
| Flash Attention | Assuming it is installed | Check availability at runtime, fall back gracefully, document installation as optional optimization |

## Performance Traps

| Trap | Symptoms | Prevention | When It Breaks |
|------|----------|------------|----------------|
| Loading model for every CLI invocation | 10-30 second startup per command | Keep model loaded across operations (daemon mode) or accept cold-start cost | Every single invocation |
| Not using inference_mode | Higher VRAM usage, slower inference | Always wrap in `torch.inference_mode()` | Immediately -- wastes 20-30% VRAM |
| Not using Flash Attention 2 | 30-40% slower inference, 20-25% more VRAM | Install and enable flash-attn when available | With 1.7B model on 8GB VRAM cards |
| Generating full audio then encoding to MP3 | Peak memory = full WAV in memory | Stream WAV chunks to MP3 encoder incrementally | Files longer than ~5 minutes of speech |
| Re-extracting voice clone prompt on every generation | Adds 2-5 seconds per generation | Cache the clone prompt in profile on first creation | Every generation after the first |

## Security Mistakes

| Mistake | Risk | Prevention |
|---------|------|------------|
| Running arbitrary Python code via PyO3 without sandboxing | If voice profile files contain Python code, code injection is possible | Never eval/exec user-provided strings; voice profiles should be data-only (JSON/TOML + audio files) |
| Storing HuggingFace tokens in profile config | Token leakage if profiles are shared | Use system keyring or HF CLI token; never store in chatter config |
| Not validating reference audio files | Malformed audio could crash model or cause undefined behavior | Validate audio format, duration, and sample rate before passing to model |

## UX Pitfalls

| Pitfall | User Impact | Better Approach |
|---------|-------------|-----------------|
| No GPU detection at startup | Users without CUDA get cryptic Python traceback | Check `torch.cuda.is_available()` first, print "CUDA GPU required" and exit cleanly |
| Silent model download on first run | Users think tool is broken, kill it, corrupt cache | Explicit "Downloading model..." with progress bar and estimated time |
| No voice preview before saving | Users save a voice design they have not heard | Always play or save a sample during voice design, require confirmation |
| Unclear error when Python not found | Raw PyO3 panic or linker error | Detect Python availability at startup, print setup instructions |
| Progress bar during inference shows no ETA | Users do not know if generation will take 10 seconds or 10 minutes | Use token generation rate to estimate remaining time |
| 0.6B model recommended for speed but produces poor quality on long text | Users choose speed, get garbage output | Default to 1.7B, warn about 0.6B limitations for long text in help text |

## "Looks Done But Isn't" Checklist

- [ ] **Voice Design:** Often missing the clone-prompt caching step -- verify that `create_voice_clone_prompt` output is stored in the profile, not just the audio
- [ ] **File Processing:** Often missing text chunking -- verify that PDFs longer than 500 characters are split before inference
- [ ] **Error Handling:** Often missing GPU OOM recovery -- verify that CUDA OOM produces a user-friendly message, not a Python traceback
- [ ] **Environment Setup:** Often missing Python version validation -- verify that `python --version` returns 3.10.x at startup
- [ ] **MP3 Output:** Often missing sample rate mismatch handling -- verify that model output sample rate matches MP3 encoder input expectation
- [ ] **Progress Bars:** Often missing for model download -- verify first-run experience shows download progress, not just inference progress
- [ ] **Profile Storage:** Often missing atomic writes -- verify that interrupted profile saves do not corrupt existing profiles

## Recovery Strategies

| Pitfall | Recovery Cost | Recovery Steps |
|---------|---------------|----------------|
| Corrupted HuggingFace model cache | LOW | Delete `~/.cache/huggingface/hub/models--Qwen--Qwen3-TTS*`, re-download |
| GIL deadlock in production | MEDIUM | Redesign threading model to single Python thread with channel communication |
| Dependency version conflict | MEDIUM | Create fresh venv, pin all versions in requirements.txt, test clean install |
| Voice profiles missing clone prompt | LOW | Add migration that re-extracts clone prompts from stored audio samples |
| GPU memory leak across invocations | MEDIUM | Add explicit `torch.cuda.empty_cache()` + `gc.collect()` after each inference call |
| Long text infinite loop | LOW | Add generation timeout, kill and retry with smaller chunk size |

## Pitfall-to-Phase Mapping

| Pitfall | Prevention Phase | Verification |
|---------|------------------|--------------|
| Dependency hell | Phase 1 (Foundation) | Clean venv install succeeds on CI with pinned versions |
| Venv detection failure | Phase 1 (Foundation) | Binary works when run outside the build environment |
| GIL deadlocks | Phase 2 (Core TTS) | Progress bars update during inference without hangs |
| GPU memory leaks | Phase 2 (Core TTS) | Run 10 sequential generations, VRAM stays stable |
| Voice profile reuse | Phase 2 (Voice Profiles) | Same profile produces same voice across multiple generations |
| Long text drift/hangs | Phase 3 (File Processing) | 10-page PDF generates without speed drift or hangs |
| First-run download UX | Phase 1 (Foundation) | First run shows progress bar, interrupted download recovers |
| Cross-platform Python | Phase 4 (Polish) | Binary works on macOS and Linux without manual Python config |

## Sources

- [PyO3 GIL deadlock discussion #3045](https://github.com/PyO3/pyo3/discussions/3045)
- [PyO3 GIL deadlock discussion #3089](https://github.com/PyO3/pyo3/discussions/3089)
- [PyO3 memory leak issue #1547](https://github.com/PyO3/pyo3/issues/1547)
- [PyO3 memory leak issue #2853](https://github.com/PyO3/pyo3/issues/2853)
- [PyO3 venv detection discussion #3726](https://github.com/PyO3/pyo3/discussions/3726)
- [PyO3 PYO3_PYTHON venv ignored #4841](https://github.com/PyO3/pyo3/issues/4841)
- [PyO3 Building and Distribution docs](https://pyo3.rs/main/building-and-distribution.html)
- [PyO3 FAQ and Troubleshooting](https://pyo3.rs/main/faq)
- [Qwen3-TTS transformers version conflict #237](https://github.com/QwenLM/Qwen3-TTS/issues/237)
- [Qwen3-TTS Python dependency issue #145](https://github.com/QwenLM/Qwen3-TTS/issues/145)
- [Qwen3-TTS inconsistent speaking rate #239](https://github.com/QwenLM/Qwen3-TTS/issues/239)
- [Qwen3-TTS slow inference #89](https://github.com/QwenLM/Qwen3-TTS/issues/89)
- [Qwen3-TTS voice persistence discussion #220](https://github.com/QwenLM/Qwen3-TTS/discussions/220)
- [Qwen3-TTS Hardware Guide](https://deepwiki.com/mu-zi-lee/qwen3-tts-skill/8.2-memory-and-hardware-requirements)
- [PyTorch torch.no_grad memory leak #49401](https://github.com/pytorch/pytorch/issues/49401)
- [PyTorch GPU memory during inference](https://discuss.pytorch.org/t/releasing-memory-after-running-a-pytorch-model-inference/175654)
- [mp3lame-encoder Rust crate](https://crates.io/crates/mp3lame-encoder)
- [Qwen3-TTS Voice Cloning Guide](https://ocdevel.com/blog/20260302-qwen-tts-voice-cloning)
- [Qwen3-TTS CustomVoice model card](https://huggingface.co/Qwen/Qwen3-TTS-12Hz-1.7B-CustomVoice)

---
*Pitfalls research for: Rust CLI with PyO3 + Qwen3-TTS*
*Researched: 2026-03-27*
