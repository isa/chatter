#!/usr/bin/env bash
#
# Test the Homebrew formula locally using a local tap.
#
# Creates a tarball from the working tree, generates a formula that points
# to it via file:// URL, and runs `brew install`. This always builds from
# the current source — no stale GitHub downloads.
#
# Usage:
#   ./scripts/brew-test-local.sh                     # full install test (quiet mode)
#   ./scripts/brew-test-local.sh --verbose           # show full brew output
#   ./scripts/brew-test-local.sh --audit             # also run brew audit
#   ./scripts/brew-test-local.sh --runtime-bundle FILE.tar.gz
#
set -euo pipefail

AUDIT=false
VERBOSE=false
RUNTIME_BUNDLE=""
DEBUG_LOG_PATH="/Users/isa.goksu/Projects/playgrounds/rust/chatter/.cursor/debug-5d112f.log"
DEBUG_SESSION_ID="5d112f"
DEBUG_RUN_ID="brew-test-local-$(date +%s)"

debug_log() {
  local hypothesis_id="$1"
  local location="$2"
  local message="$3"
  local data_json="${4:-{}}"
  local ts
  ts="$(python3 - <<'PY'
import time
print(int(time.time() * 1000))
PY
)"
  printf '{"sessionId":"%s","runId":"%s","hypothesisId":"%s","location":"%s","message":"%s","data":%s,"timestamp":%s}\n' \
    "$DEBUG_SESSION_ID" "$DEBUG_RUN_ID" "$hypothesis_id" "$location" "$message" "$data_json" "$ts" >> "$DEBUG_LOG_PATH"
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --audit)
      AUDIT=true
      shift
      ;;
    --verbose)
      VERBOSE=true
      shift
      ;;
    --runtime-bundle)
      RUNTIME_BUNDLE="${2:-}"
      if [[ -z "$RUNTIME_BUNDLE" ]]; then
        echo "error: --runtime-bundle requires a file path"
        exit 1
      fi
      if [[ ! -f "$RUNTIME_BUNDLE" ]]; then
        echo "error: runtime bundle not found: $RUNTIME_BUNDLE"
        exit 1
      fi
      shift 2
      ;;
    *)
      echo "error: unknown option: $1"
      exit 1
      ;;
  esac
done

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
VERSION=$(grep '^version' "$REPO_ROOT/Cargo.toml" | head -1 | sed 's/.*"\(.*\)".*/\1/')
TAP_NAME="local/chatter"
TAP_DIR="$(brew --repository)/Library/Taps/local/homebrew-chatter"

#region agent log
debug_log "H1" "brew-test-local.sh:setup" "script_start" "{\"version\":\"${VERSION}\",\"tap_name\":\"${TAP_NAME}\",\"tap_dir\":\"${TAP_DIR}\"}"
#endregion

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

# This tap has no origin remote: brew update skips it (expected).
# Re-run this script to refresh; for GitHub-tracked updates use: brew tap isa/tap

echo "==> Uninstalling previous version (if any)..."
if brew uninstall chatter 2>/dev/null; then
  uninstall_result="removed"
else
  uninstall_result="not-installed-or-failed"
fi
#region agent log
debug_log "H2" "brew-test-local.sh:uninstall" "post_uninstall" "{\"result\":\"${uninstall_result}\"}"
#endregion

echo "==> Installing from local source..."
if [[ -n "$RUNTIME_BUNDLE" ]]; then
  export CHATTER_RUNTIME_BUNDLE_URL="file://${RUNTIME_BUNDLE}"
  echo "    Using runtime bundle: $RUNTIME_BUNDLE"
fi

if [[ "$VERBOSE" == "true" ]]; then
  brew install --verbose "${TAP_NAME}/chatter"
else
  LOG_FILE="${TARBALL_DIR}/brew-install-${VERSION}.log"
  #region agent log
  debug_log "H3" "brew-test-local.sh:install" "brew_install_started" "{\"log_file\":\"${LOG_FILE}\",\"tap_name\":\"${TAP_NAME}/chatter\"}"
  #endregion
  # Keep output short by default while still preserving full logs.
  brew install "${TAP_NAME}/chatter" >"$LOG_FILE" 2>&1 &
  BREW_PID=$!

  SPIN='-\|/'
  i=0
  while kill -0 "$BREW_PID" 2>/dev/null; do
    i=$(( (i + 1) % 4 ))
    printf "\r    Installing chatter %c" "${SPIN:$i:1}"
    sleep 0.2
  done
  set +e
  wait "$BREW_PID"
  install_exit=$?
  set -e
  #region agent log
  debug_log "H3" "brew-test-local.sh:install" "brew_install_finished" "{\"exit_code\":${install_exit},\"log_file\":\"${LOG_FILE}\"}"
  #endregion
  if [[ "$install_exit" -ne 0 ]]; then
    install_linkage_conflict=false
    if rg -q "Failed to fix install linkage" "$LOG_FILE" && rg -q "Formulae found in multiple taps" "$LOG_FILE"; then
      install_linkage_conflict=true
    fi
    cellar_bin="$(brew --cellar)/chatter/${VERSION}/bin/chatter"
    if [[ "$install_linkage_conflict" == "true" && -x "$cellar_bin" ]]; then
      #region agent log
      debug_log "H4" "brew-test-local.sh:install" "linkage_conflict_recovered" "{\"cellar_bin\":\"${cellar_bin}\",\"install_exit\":${install_exit}}"
      #endregion
      echo ""
      echo "    brew install returned ${install_exit} due to linkage conflict (multiple taps), but keg exists."
      echo "    Continuing with Cellar binary: $cellar_bin"
    else
      echo ""
      echo "    brew install failed (exit ${install_exit})"
      echo "    Full install log: $LOG_FILE"
      echo "    (Run with --verbose to stream errors live.)"
      exit "$install_exit"
    fi
  fi
  printf "\r    Installing chatter done\n"
  echo "    Full install log: $LOG_FILE"
fi

echo ""
echo "==> Running chatter doctor (simulating clean user environment)..."
# Unset CHATTER_VENV to simulate a real brew user who doesn't have it
unset CHATTER_VENV 2>/dev/null || true
CHATTER_BIN="$(brew --prefix)/bin/chatter"
if [[ ! -x "$CHATTER_BIN" ]]; then
  CHATTER_BIN="$(brew --cellar)/chatter/${VERSION}/bin/chatter"
fi
#region agent log
debug_log "H5" "brew-test-local.sh:doctor" "resolved_chatter_bin" "{\"chatter_bin\":\"${CHATTER_BIN}\"}"
#endregion
"$CHATTER_BIN" doctor || true

echo ""
echo "==> Running brew test..."
brew test "${TAP_NAME}/chatter" || true

if [[ "$AUDIT" == "true" ]]; then
    echo ""
    echo "==> Running brew audit..."
    brew audit --strict "${TAP_DIR}/Formula/chatter.rb" || true
fi

echo ""
echo "==> Install location:"
if [[ -x "$(brew --prefix)/bin/chatter" ]]; then
  ls -la "$(brew --prefix)/bin/chatter"
else
  echo "  (not linked in $(brew --prefix)/bin due to multi-tap conflict)"
  ls -la "$(brew --cellar)/chatter/${VERSION}/bin/chatter" 2>/dev/null || echo "  (cellar binary not found)"
fi
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
