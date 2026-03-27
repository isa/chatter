# Phase 1: Foundation and Python Bridge - Context

**Gathered:** 2026-03-27
**Status:** Ready for planning

<domain>
## Phase Boundary

Deliver a working Rust CLI binary (`chatter`) that initializes a Python runtime via PyO3, loads Qwen3-TTS, validates the user's environment, and exposes all planned subcommands in `--help`. Users can confirm their system is ready for TTS work. Model management subcommands (download, list, load) are included in this phase.

</domain>

<decisions>
## Implementation Decisions

### Model Loading Timing
- **D-01:** Lazy model loading — only load the model when a command actually needs inference (design, clone, generate). Non-inference commands (`--help`, `profiles list`, `doctor`) stay instant.
- **D-02:** When model loading is triggered, show an explicit progress spinner with message: "Loading Qwen3-TTS 1.7B..."

### Model Management
- **D-03:** Include `chatter model download/list/load` subcommands in Phase 1 for explicit offline model management. Users should be able to pre-download models rather than waiting for first inference.
- **D-04:** All planned subcommands (design, clone, generate, profiles, model, doctor) visible in `--help` from the start. Unimplemented ones (design, clone, generate) return a "coming in Phase 2" message.

### Error Presentation
- **D-05:** Minimal errors by default — short, one-line error messages. `--verbose` flag reveals full diagnostic with system info.
- **D-06:** Color-coded errors — red for errors, yellow for warnings. Respects `NO_COLOR` env var per standard (using owo-colors).

### Progress Feedback
- **D-07:** Spinner with status message style (indicatif). E.g., "Loading Qwen3-TTS 1.7B..."
- **D-08:** Elapsed time shown alongside spinner: "Loading Qwen3-TTS 1.7B... (12s)" so users know the process isn't stuck.

### Environment Validation
- **D-09:** Dedicated `chatter doctor` subcommand for proactive environment checking. Inline commands just fail with short errors (per D-05).
- **D-10:** `chatter doctor` performs full system report: Python version, qwen-tts version, compute backend (MLX or CUDA), GPU/accelerator name, VRAM, PyTorch version. Pass/fail per item with green checkmarks or red X.

### Compute Backend
- **D-11:** Support both MLX (Apple Silicon/Mac) and CUDA (Linux/cloud) as compute backends. Runtime detection — use whichever is available.
- **D-12:** Update PROJECT.md constraint from "CUDA-capable GPU required" to "Apple Silicon (MLX) or CUDA-capable GPU required."

### Claude's Discretion
- Specific clap subcommand structure and flag naming
- PyO3 initialization patterns and error handling internals
- Project directory layout (src/ organization)
- How "coming soon" messages are worded for unimplemented subcommands

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Project Definition
- `.planning/PROJECT.md` — Core value, constraints, key decisions, context about Qwen3-TTS
- `.planning/REQUIREMENTS.md` — FOUN-01 through FOUN-05, UX-01, UX-04 are Phase 1 requirements
- `.planning/ROADMAP.md` — Phase 1 success criteria and dependency chain

### Technology Stack
- `CLAUDE.md` §Technology Stack — Full recommended stack with versions, PyO3 integration strategy, audio pipeline, what NOT to use

### External References
- [Qwen3-TTS GitHub](https://github.com/QwenLM/Qwen3-TTS) — Model documentation, usage patterns
- [qwen-tts PyPI](https://pypi.org/project/qwen-tts/) — Python package API
- [PyO3 User Guide](https://pyo3.rs/) — Embedding patterns, auto-initialize feature

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- None — greenfield project. Only CLAUDE.md, LICENSE, README.md exist.

### Established Patterns
- None yet. Phase 1 establishes all patterns.

### Integration Points
- Python 3.12+ runtime must be available at build time (PyO3 build config)
- `qwen-tts` Python package must be importable at runtime
- MLX framework must be available on Mac; CUDA/PyTorch on Linux

</code_context>

<specifics>
## Specific Ideas

- User is on Mac — MLX is the primary development/testing backend. CUDA support must work but Mac is the daily driver.
- Model management commands should support offline workflows (pre-download models before use).

</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope.

Note: The addition of model management subcommands was discussed and explicitly included in Phase 1 scope by user decision (D-03, D-04).

</deferred>

---

*Phase: 01-foundation-and-python-bridge*
*Context gathered: 2026-03-27*
