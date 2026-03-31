---
quick_task: 260331-5dj
date: 2026-03-31
status: complete
---

# Quick Task 260331-5dj Summary

Implemented a practical speed/noise improvement path for Homebrew installs:

- Formula supports optional preconfigured runtime bundle via `CHATTER_RUNTIME_BUNDLE_URL` and falls back to current behavior.
- Pip output is quieter by default (`PIP_PROGRESS_BAR=off`, `--quiet` on pip installs).
- `scripts/brew-test-local.sh` now runs quiet by default with a spinner and saves full logs; `--verbose` is still available.
- Added `scripts/build-runtime-bundle.sh` to produce a reusable `venv` tarball for maintainers/CI.
- Documented the approach in `README.md`.
