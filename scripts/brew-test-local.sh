#!/usr/bin/env bash
#
# Test the Homebrew formula locally using a local tap.
#
# Usage:
#   ./scripts/brew-test-local.sh          # full install test
#   ./scripts/brew-test-local.sh --audit  # also run brew audit
#
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
VERSION=$(grep '^version' "$REPO_ROOT/Cargo.toml" | head -1 | sed 's/.*"\(.*\)".*/\1/')
TARBALL_NAME="chatter-${VERSION}.tar.gz"
TAP_NAME="local/chatter"
TAP_DIR="$(brew --repository)/Library/Taps/local/homebrew-chatter"

echo "==> Creating source tarball (v${VERSION})..."
CACHE_DIR="${HOMEBREW_CACHE:-$(brew --cache)}"
tar -czf "${CACHE_DIR}/${TARBALL_NAME}" \
    --exclude='.git' \
    --exclude='target' \
    --exclude='.planning' \
    --exclude='__pycache__' \
    -C "$(dirname "$REPO_ROOT")" \
    "$(basename "$REPO_ROOT")"

echo "==> Setting up local tap at ${TAP_DIR}..."
mkdir -p "${TAP_DIR}/Formula"
# Initialize as a git repo (Homebrew requires taps to be git repos)
if [ ! -d "${TAP_DIR}/.git" ]; then
    git -C "${TAP_DIR}" init -q
fi
cp "$REPO_ROOT/Formula/chatter.rb" "${TAP_DIR}/Formula/chatter.rb"
git -C "${TAP_DIR}" add -A && git -C "${TAP_DIR}" commit -q -m "update formula" --allow-empty 2>/dev/null || true

echo "==> Uninstalling previous version (if any)..."
brew uninstall chatter 2>/dev/null || true

echo "==> Clearing Homebrew download cache for chatter..."
rm -f "${CACHE_DIR}/chatter--${VERSION}"*.tar.gz
rm -f "${CACHE_DIR}/downloads/"*"--chatter-${VERSION}.tar.gz"

echo "==> Installing from local tap..."
brew install --verbose "${TAP_NAME}/chatter"

echo ""
echo "==> Running chatter doctor (simulating clean user environment)..."
# Unset CHATTER_VENV to simulate a real brew user who doesn't have it
unset CHATTER_VENV
"$(brew --prefix)/bin/chatter" doctor || true

echo ""
echo "==> Running brew test..."
brew test "${TAP_NAME}/chatter" || true

if [[ "${1:-}" == "--audit" ]]; then
    echo ""
    echo "==> Running brew audit..."
    brew audit --strict "${TAP_DIR}/Formula/chatter.rb" || true
fi

echo ""
echo "==> Install location:"
ls -la "$(brew --prefix)/bin/chatter"
echo ""
echo "==> Venv location:"
ls -la "$(brew --cellar)/chatter/${VERSION}/libexec/venv/bin/python" 2>/dev/null || echo "  (venv not found — check formula)"
echo ""
echo "==> Done."
echo "    To uninstall:  brew uninstall chatter"
echo "    To remove tap:  brew untap ${TAP_NAME}"
