#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
OUT_DIR="${TMPDIR:-/tmp}/chatter-runtime-bundle"
WORK_DIR="${OUT_DIR}/work"
BUNDLE_PATH="${OUT_DIR}/chatter-runtime-venv-macos-arm64.tar.gz"

PYTHON_BIN="${PYTHON_BIN:-python3.13}"

echo "==> Building preconfigured runtime venv bundle"
echo "    Python: $PYTHON_BIN"
echo "    Output: $BUNDLE_PATH"

rm -rf "$WORK_DIR"
mkdir -p "$WORK_DIR"

"$PYTHON_BIN" -m venv "${WORK_DIR}/venv"
PIP="${WORK_DIR}/venv/bin/pip"

PIP_DISABLE_PIP_VERSION_CHECK=1 PIP_PROGRESS_BAR=off \
  "$PIP" install --no-cache-dir --quiet --only-binary :all: \
  -r "${REPO_ROOT}/requirements-mlx.txt"

tar -czf "$BUNDLE_PATH" -C "$WORK_DIR" venv

echo "==> Done"
echo "    Bundle: $BUNDLE_PATH"
