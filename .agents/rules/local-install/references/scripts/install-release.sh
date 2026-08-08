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
  Darwin-arm64)  ASSET="ktlint-rs-aarch64-apple-darwin.tar.gz" ;;
  Darwin-x86_64) ASSET="ktlint-rs-x86_64-apple-darwin.tar.gz" ;;
  Linux-x86_64)  ASSET="ktlint-rs-x86_64-unknown-linux-gnu.tar.gz" ;;
  MINGW*|MSYS*)  ASSET="ktlint-rs-x86_64-pc-windows-msvc.zip" ;;
  *) echo "unsupported platform: $(uname -s)-$(uname -m)" >&2; exit 1 ;;
esac

URL="https://github.com/$REPO/releases/download/$TAG/$ASSET"
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

echo "fetching $URL"
case "$ASSET" in
  *.zip) curl -fsSL "$URL" -o "$TMP/kt.zip" && unzip -o -q "$TMP/kt.zip" -d "$TMP" ;;
  *.tar.gz) curl -fsSL "$URL" -o "$TMP/kt.tar.gz" && tar xzf "$TMP/kt.tar.gz" -C "$TMP" ;;
esac

BIN="$(find "$TMP" -type f -name "ktlint-rs" -o -type f -name "ktlint-rs.exe" | head -1)"
[ -n "$BIN" ] || { echo "ktlint-rs binary not found in asset" >&2; exit 1; }

mkdir -p "$DEST"
install -m 755 "$BIN" "$DEST/ktlint-rs"
echo "installed: $DEST/ktlint-rs"
"$DEST/ktlint-rs" --version
