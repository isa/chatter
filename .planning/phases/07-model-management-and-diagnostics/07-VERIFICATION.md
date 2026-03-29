---
phase: 07-model-management-and-diagnostics
verified: 2026-03-29T22:00:00Z
status: gaps_found
score: 9/11 must-haves verified
gaps:
  - truth: "chatter doctor shows ChatterBox-compatible hardware (MPS/CUDA) status"
    status: partial
    reason: "The doctor command surfaces GPU hardware (MLX/MPS/CUDA) but it is shown in the top-level Compute Backend section, not repeated or labeled in the ChatterBox section. The plan required a hardware check scoped to ChatterBox. The existing GPU check technically covers it but is not associated with the ChatterBox section header."
    artifacts:
      - path: "src/commands/doctor.rs"
        issue: "No dedicated ChatterBox hardware check inside the ChatterBox section (lines 165-208). The GPU check at lines 104-133 is a shared check above both engine sections."
    missing:
      - "Either move/duplicate the GPU check inside the ChatterBox section, or add a note under ChatterBox referencing hardware compatibility. Severity is low because GPU info is shown, just not co-located with ChatterBox diagnostics."

  - truth: "chatter doctor --fix installs ChatterBox deps and models alongside Qwen"
    status: failed
    reason: "--fix handler calls install_chatterbox_via_pip() (pip subprocess) but never calls download_model_chatterbox(). Models are not downloaded during --fix, only the Python package is installed."
    artifacts:
      - path: "src/commands/doctor.rs"
        issue: "Lines 282-311: install_chatterbox_via_pip() is called but download_model_chatterbox() is never invoked. The plan key_link 'install_chatterbox_deps() call in --fix handler' was replaced with a local pip function, and model download step was omitted entirely."
    missing:
      - "After install_chatterbox_via_pip() succeeds, call bridge::model::download_model_chatterbox() to download ChatterBox models. Wrap errors as warnings (ChatterBox is optional) and do not hard-fail."
---

# Phase 7: Model Management and Diagnostics Verification Report

**Phase Goal:** Users can download, list, and diagnose ChatterBox models through existing CLI commands
**Verified:** 2026-03-29T22:00:00Z
**Status:** gaps_found
**Re-verification:** No — initial verification

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | `chatter model list` groups models by engine (Qwen section, ChatterBox section) | VERIFIED | `src/commands/model.rs` lines 128-183: partitions models by `engine` field, prints "Qwen3-TTS Models" and "ChatterBox Models" section headers with separators |
| 2 | `chatter model list` shows variant name (Original, Turbo, Multilingual) alongside repo ID for ChatterBox | VERIFIED | `src/commands/model.rs` line 179: `model.variant_label.as_deref().unwrap_or(&model.repo_id)` — variant labels derived in `chatterbox_variant_label()` at `src/bridge/model.rs` lines 120-132 |
| 3 | `chatter model download --engine chatterbox` shows estimated total size and available disk space before starting | VERIFIED | `src/commands/model.rs` lines 55-74: calls `disk_space_check()`, prints "Estimated download size:" and "Available disk space:" before invoking install/download |
| 4 | `chatter model download --engine chatterbox` warns if less than 5GB would remain after download | VERIFIED | `src/commands/model.rs` lines 59-64: `if free < estimated + 5_000_000_000` → prints yellow "Warning: Less than 5 GB will remain after download." |
| 5 | `chatter doctor` shows ChatterBox installation status alongside Qwen3-TTS checks | VERIFIED | `src/commands/doctor.rs` lines 165-208: dedicated "ChatterBox" section with bold header, chatterbox-tts package check, CB Models check |
| 6 | `chatter doctor` shows ChatterBox package version when installed | VERIFIED | `src/commands/doctor.rs` lines 175-180: `ui::doctor_pass("chatterbox-tts", version)` where version comes from `info.chatterbox_pkg_version` |
| 7 | `chatter doctor` shows ChatterBox-compatible hardware (MPS/CUDA) status | PARTIAL | GPU check exists at lines 104-133 and is general (covers MLX/MPS/CUDA/CPU), but is not placed inside or labeled for the ChatterBox section. No dedicated hardware check appears under the "ChatterBox" section header. |
| 8 | `chatter doctor` shows disk space check relevant to ChatterBox models | VERIFIED | `src/commands/doctor.rs` lines 220-233: disk space check shows free GB; warnings fire below 10 GB threshold |
| 9 | `chatter doctor --fix` installs ChatterBox deps and models alongside Qwen | FAILED | `src/commands/doctor.rs` lines 282-311: `install_chatterbox_via_pip()` runs but `download_model_chatterbox()` is never called — models are not downloaded |
| 10 | If ChatterBox not installed, doctor shows informational 'not installed' (not a failure) | VERIFIED | `src/commands/doctor.rs` lines 183-186: `ui::doctor_warn("chatterbox-tts", "not installed (optional...")` — does not increment `fails` |
| 11 | `download_model_chatterbox()` is actually wired into `chatter model download --engine chatterbox` | VERIFIED | `src/commands/model.rs` line 85: `bridge::model::download_model_chatterbox()` is called after `install_chatterbox_deps()` succeeds |

**Score:** 9/11 truths verified (1 partial, 1 failed)

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `src/bridge/model.rs` | `disk_space_check()` function, `variant_label()` helper, `engine` field on `ModelInfo` | VERIFIED | All three present: `ModelInfo` has `engine: String` (line 41) and `variant_label: Option<String>` (line 43); `disk_space_check()` at line 138; `chatterbox_variant_label()` at line 120 |
| `src/commands/model.rs` | Engine-grouped list output, disk space pre-check before ChatterBox download | VERIFIED | "Qwen3-TTS Models" and "ChatterBox Models" section headers present; `disk_space_check()` called at line 55 before download |
| `src/bridge/doctor.rs` | ChatterBox package version and installation check in `SystemInfo` | VERIFIED | `SystemInfo` has `chatterbox_pkg_version: Option<String>` (line 16) and `chatterbox_installed: bool` (line 18); `get_system_info()` populates both (lines 49-50) |
| `src/commands/doctor.rs` | ChatterBox section in doctor output, extended --fix | PARTIAL | ChatterBox section exists and is correct; --fix installs package but does not download models |

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| `src/commands/model.rs` | `src/bridge/model.rs` | `disk_space_check()` call before `download_model_chatterbox()` | WIRED | `disk_space_check()` called at line 55, `download_model_chatterbox()` called at line 85 — correct order |
| `src/commands/doctor.rs` | `src/bridge/doctor.rs` | `SystemInfo.chatterbox_pkg_version` field | WIRED | `info.chatterbox_pkg_version` accessed at line 177; `info.chatterbox_installed` accessed at lines 175, 190 |
| `src/commands/doctor.rs` | `src/bridge/venv.rs` | `install_chatterbox_deps()` call in --fix handler | NOT_WIRED | The plan required `install_chatterbox_deps()` from `bridge::venv`. The actual implementation uses `install_chatterbox_via_pip()`, a local function in `commands/doctor.rs` that invokes pip directly. `bridge::venv::install_chatterbox_deps()` does exist (confirmed in `src/bridge/venv.rs`) but is not called here. Functionally equivalent, but the contract is different. |
| `src/commands/doctor.rs` | `src/bridge/model.rs` | `download_model_chatterbox()` call in --fix handler | NOT_WIRED | `--fix` only calls `install_chatterbox_via_pip()`. There is no call to `bridge::model::download_model_chatterbox()` or `bridge::list_cached_chatterbox_models()` in the fix path. |

### Data-Flow Trace (Level 4)

| Artifact | Data Variable | Source | Produces Real Data | Status |
|----------|---------------|--------|--------------------|--------|
| `src/commands/model.rs` list handler | `models` / `qwen_models` / `cb_models` | `bridge::model::list_cached_models()` → PyO3 → `huggingface_hub.scan_cache_dir()` | Yes — scans real HF cache on disk | FLOWING |
| `src/commands/doctor.rs` ChatterBox section | `info.chatterbox_pkg_version` | `get_system_info()` → `get_package_version(py, "chatterbox-tts")` → `importlib.metadata.version()` | Yes — queries live Python package metadata | FLOWING |
| `src/commands/doctor.rs` CB Models check | `bridge::list_cached_chatterbox_models()` | PyO3 → `huggingface_hub.scan_cache_dir()` filtered by ResembleAI/chatterbox prefix | Yes — real cache scan | FLOWING |

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| `cargo check` compilation | `cargo check 2>&1 \| tail -5` | `Finished 'dev' profile ... in 0.53s` — 9 warnings, 0 errors | PASS |
| `disk_space_check()` function exists | `grep -n "pub fn disk_space_check" src/bridge/model.rs` | Line 138 found | PASS |
| "ChatterBox Models" section header present | `grep -n "ChatterBox Models" src/commands/model.rs` | Line 161 found | PASS |
| `chatterbox_pkg_version` field in SystemInfo | `grep -n "chatterbox_pkg_version" src/bridge/doctor.rs` | Lines 16, 49 found | PASS |
| `download_model_chatterbox` in --fix doctor path | `grep -n "download_model_chatterbox" src/commands/doctor.rs` | Not found | FAIL |

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|------------|-------------|--------|----------|
| MDL-01 | 07-01-PLAN.md | `chatter model` command supports downloading and listing ChatterBox model variants (Original, Turbo, Multilingual) | SATISFIED | Model list shows grouped sections with variant labels. Download path calls `download_model_chatterbox()`. |
| MDL-02 | 07-02-PLAN.md | `chatter doctor` validates ChatterBox installation (package version, MPS/CUDA availability, disk space) | PARTIAL | Package version check and disk space covered. Hardware check exists but not co-located in ChatterBox section. |
| MDL-03 | 07-01-PLAN.md | Model download shows disk space requirements and warns before downloading large models | SATISFIED | `disk_space_check()` called, sizes printed, 5GB warning shown before download. |

**REQUIREMENTS.md traceability table** lists MDL-01 as "Complete", MDL-02 as "Pending", MDL-03 as "Complete" — consistent with this verification finding.

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| `src/commands/doctor.rs` | 294 | `install_chatterbox_via_pip()` only — no `download_model_chatterbox()` call in --fix | Blocker | `--fix` does not complete the stated goal of downloading ChatterBox models; only installs the Python package |
| `src/bridge/venv.rs` | 286 | `is_chatterbox_installed()` unused (compiler warning) | Warning | Dead code, not a goal blocker |
| `src/commands/doctor.rs` | 284 | `get_system_info_chatterbox_installed()` called twice (expensive Python GIL attach) | Info | Redundant Python round-trip; not a correctness issue |

### Human Verification Required

#### 1. ChatterBox hardware section placement

**Test:** Run `chatter doctor` on a machine without ChatterBox installed (typical dev setup).
**Expected:** The "ChatterBox" section should make it clear whether the hardware is compatible, not require the user to cross-reference the top-level GPU check.
**Why human:** Requires subjective judgment on whether the current placement (GPU check above both engine sections) is acceptable UX or whether the ChatterBox section needs its own hardware line.

#### 2. `--fix` end-to-end flow

**Test:** On a clean machine, run `chatter doctor --fix`.
**Expected:** Should install ChatterBox Python package AND download ChatterBox model variants (Original, Turbo, Multilingual).
**Why human:** Cannot verify the download step works without a real environment; automated checks confirm it is not wired in the code.

### Gaps Summary

Two gaps block full goal achievement:

**Gap 1 (Partial — low severity):** The "ChatterBox" section in `chatter doctor` lacks a dedicated hardware compatibility line. The GPU check is present in the output but lives above both engine sections, not inside the ChatterBox block. The plan specified hardware status "in the ChatterBox section" per D-04. This is a UX placement issue, not a missing check.

**Gap 2 (Failed — medium severity):** `chatter doctor --fix` installs the ChatterBox Python package (`chatterbox-tts`) via pip but never downloads the ChatterBox model files. The `--fix` handler calls `install_chatterbox_via_pip()` and then proceeds directly to the "Fix complete." message without invoking `bridge::model::download_model_chatterbox()`. A user running `chatter doctor --fix` would have the package installed but no models downloaded, and would need to run `chatter model download --engine chatterbox` separately. This contradicts the plan truth "chatter doctor --fix installs ChatterBox deps and models alongside Qwen."

The root cause of Gap 2 is documented in the 07-02-SUMMARY.md as a deviation: the executor used a pip subprocess for package installation but omitted the model download step entirely, not just the bridge function substitution.

---

_Verified: 2026-03-29T22:00:00Z_
_Verifier: Claude (gsd-verifier)_
