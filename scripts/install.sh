#!/usr/bin/env bash
# ktlint-rs installer for Linux and macOS.
#
# Usage:
#   curl -fsSL https://github.com/qdsfdhvh/ktlint-rs/releases/latest/download/install.sh | bash
#
# Environment variables:
#   KTLINT_RS_VERSION   pin a version (e.g. v0.1.0). Default: latest release.
#   KTLINT_RS_REPO      override the source repo (default: qdsfdhvh/ktlint-rs).
#   KTLINT_RS_PREFIX    install directory. Default: $HOME/.local/bin
#   KTLINT_RS_FORCE_BINARY  set to 1 to force GitHub Release download.
set -euo pipefail

REPO="${KTLINT_RS_REPO:-qdsfdhvh/ktlint-rs}"
REPO_URL="https://github.com/${REPO}"
VERSION="${KTLINT_RS_VERSION:-latest}"
PREFIX="${KTLINT_RS_PREFIX:-$HOME/.local/bin}"
FORCE_BINARY="${KTLINT_RS_FORCE_BINARY:-0}"

err() { printf '\033[31merror:\033[0m %s\n' "$*" >&2; exit 1; }
info() { printf '\033[36m::\033[0m %s\n' "$*"; }

# ── Detect platform ────────────────────────────────────────────────
detect_platform() {
  local arch=$(uname -m)
  local os=$(uname -s | tr '[:upper:]' '[:lower:]')
  case "$os-$arch" in
    darwin-arm64|darwin-aarch64)  echo "aarch64-apple-darwin" ;;
    darwin-x86_64)                echo "x86_64-apple-darwin" ;;
    linux-x86_64)                 echo "x86_64-unknown-linux-gnu" ;;
    linux-arm64|linux-aarch64)    echo "aarch64-unknown-linux-gnu" ;;
    *) err "Unsupported platform: $os-$arch" ;;
  esac
}

# ── Download binary from GitHub Release ─────────────────────────────
download_binary() {
  local platform=$(detect_platform)
  local tag="$VERSION"
  if [ "$tag" = "latest" ]; then
    tag=$(curl -fsSL "$REPO_URL/releases/latest" 2>/dev/null | grep -o 'tag/[^"]*' | head -1 | cut -d/ -f2)
    [ -n "$tag" ] || err "Cannot determine latest version"
  fi
  local url="${REPO_URL}/releases/download/${tag}/ktlint-rs-${platform}.tar.gz"
  info "Downloading ktlint-rs ${tag} for ${platform}..."
  mkdir -p "$PREFIX"
  curl -fsSL "$url" -o /tmp/ktlint-rs.tar.gz \
    && tar -xzf /tmp/ktlint-rs.tar.gz -C "$PREFIX" ktlint-rs \
    && chmod +x "$PREFIX/ktlint-rs" \
    && rm -f /tmp/ktlint-rs.tar.gz \
    && info "Installed ktlint-rs to $PREFIX/ktlint-rs" \
    || err "Download failed: $url"
}

# ── Main ────────────────────────────────────────────────────────────
if [ "$FORCE_BINARY" = "1" ]; then
  download_binary
else
  download_binary
fi

# Check PATH
if ! echo ":$PATH:" | grep -q ":$PREFIX:"; then
  warn "Add $PREFIX to your PATH:"
  printf '\033[33m  export PATH="%s:$PATH"\033[0m\n' "$PREFIX"
fi
