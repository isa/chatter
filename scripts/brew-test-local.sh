#!/usr/bin/env bash
#
# Test the Homebrew formula locally using a local tap.
#
# Creates a tarball from the working tree, generates a formula that points
# to it via file:// URL, and runs `brew install`. This always builds from
# the current source — no stale GitHub downloads.
#
# Usage:
#   ./scripts/brew-test-local.sh          # full install test
#   ./scripts/brew-test-local.sh --audit  # also run brew audit
#
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
VERSION=$(grep '^version' "$REPO_ROOT/Cargo.toml" | head -1 | sed 's/.*"\(.*\)".*/\1/')
TAP_NAME="local/chatter"
TAP_DIR="$(brew --repository)/Library/Taps/local/homebrew-chatter"

# Use a stable location for the tarball so brew can find it
TARBALL_DIR="${TMPDIR:-/tmp}/chatter-brew-test"
mkdir -p "$TARBALL_DIR"
TARBALL_PATH="${TARBALL_DIR}/chatter-${VERSION}.tar.gz"

echo "==> Creating source tarball (v${VERSION}) from working tree..."
# Create tarball with a top-level directory name that matches what brew expects
tar -czf "$TARBALL_PATH" \
    --exclude='.git' \
    --exclude='target' \
    --exclude='.planning' \
    --exclude='__pycache__' \
    --exclude='.claude/worktrees' \
    -s "|^|chatter-${VERSION}/|" \
    -C "$REPO_ROOT" .

TARBALL_SHA=$(shasum -a 256 "$TARBALL_PATH" | awk '{print $1}')
echo "    Tarball: $TARBALL_PATH"
echo "    SHA256:  $TARBALL_SHA"

echo "==> Generating local formula..."
mkdir -p "${TAP_DIR}/Formula"

# Initialize as a git repo (Homebrew requires taps to be git repos)
if [ ! -d "${TAP_DIR}/.git" ]; then
    git -C "${TAP_DIR}" init -q
fi

# Generate formula from the production one, replacing url/sha256 with local file
sed \
    -e "s|url \"https://.*|url \"file://${TARBALL_PATH}\"|" \
    -e "s|sha256 \".*|sha256 \"${TARBALL_SHA}\"|" \
    "$REPO_ROOT/Formula/chatter.rb" > "${TAP_DIR}/Formula/chatter.rb"

git -C "${TAP_DIR}" add -A && git -C "${TAP_DIR}" commit -q -m "update formula" --allow-empty 2>/dev/null || true

echo "==> Uninstalling previous version (if any)..."
brew uninstall chatter 2>/dev/null || true

echo "==> Installing from local source..."
brew install --verbose "${TAP_NAME}/chatter"

echo ""
echo "==> Running chatter doctor (simulating clean user environment)..."
# Unset CHATTER_VENV to simulate a real brew user who doesn't have it
unset CHATTER_VENV 2>/dev/null || true
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
echo "    Binary:     $(brew --prefix)/bin/chatter"
echo ""
echo "==> PATH check (read this if \`chatter --version\` is wrong):"
echo "    If ~/.cargo/bin appears before Homebrew in PATH, \`chatter\` runs the"
echo "    Cargo-installed binary, not the formula. Compare:"
echo "      which -a chatter"
echo "      $(brew --prefix)/bin/chatter --version"
echo ""
echo "    To test:    $(brew --prefix)/bin/chatter doctor"
echo "    To uninstall:  brew uninstall chatter"
echo "    To remove tap:  brew untap ${TAP_NAME}"
