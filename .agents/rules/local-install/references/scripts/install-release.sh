#!/usr/bin/env bash
# Official install: fetch a ktlint-rs binary from the GitHub release (never
# from a local build). Usage:
#   references/scripts/install-release.sh [tag] [install-dir]
# tag defaults to latest; install-dir defaults to ~/.cargo/bin.
set -euo pipefail

REPO="qdsfdhvh/ktlint-rs"
TAG="${1:-latest}"
DEST="${2:-$HOME/.cargo/bin}"

# Map the running platform to a release asset.
case "$(uname -s)-$(uname -m)" in
  Darwin-arm64)  ASSET="ktlint-rs-macos-arm64" ;;
  Darwin-x86_64) ASSET="ktlint-rs-macos-x86_64" ;;
  Linux-x86_64)  ASSET="ktlint-rs-linux-x86_64" ;;
  MINGW*|MSYS*)  ASSET="ktlint-rs-windows-x86_64.exe" ;;
  *) echo "unsupported platform: $(uname -s)-$(uname -m)" >&2; exit 1 ;;
esac

URL="https://github.com/$REPO/releases/download/$TAG/$ASSET"
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

echo "fetching $URL"
curl -fsSL "$URL" -o "$TMP/ktlint-rs"
BIN="$TMP/ktlint-rs"
[ -s "$BIN" ] || { echo "ktlint-rs binary not found in asset" >&2; exit 1; }

mkdir -p "$DEST"
install -m 755 "$BIN" "$DEST/ktlint-rs"
echo "installed: $DEST/ktlint-rs"
"$DEST/ktlint-rs" --version
