#!/usr/bin/env bash
# ktlint-rs installer for Linux and macOS.
#
# Usage:
#   curl -fsSL https://github.com/qdsfdhvh/ktlint-rs/releases/latest/download/install.sh | bash
#
# Tries in order:
#   1. GitHub Release binary download (no Rust toolchain needed)
#   2. cargo install (fallback for Rust developers)
#
# Environment variables:
#   KTLINT_RS_VERSION   pin a version (e.g. v0.1.0). Default: latest release.
#   KTLINT_RS_REPO      override the source repo (default: qdsfdhvh/ktlint-rs).
#   KTLINT_RS_PREFIX    install directory. Default: $HOME/.local/bin
set -euo pipefail

REPO="${KTLINT_RS_REPO:-qdsfdhvh/ktlint-rs}"
REPO_URL="https://github.com/${REPO}"
VERSION="${KTLINT_RS_VERSION:-latest}"
PREFIX="${KTLINT_RS_PREFIX:-$HOME/.local/bin}"

err() { printf '\033[31merror:\033[0m %s\n' "$*" >&2; exit 1; }
info() { printf '\033[36m::\033[0m %s\n' "$*"; }
warn() { printf '\033[33m!\033[0m %s\n' "$*"; }

# ── binary download ────────────────────────────────────────────────
download_binary() {
  local uname_s="$(uname -s)" uname_m="$(uname -m)"
  local os="" arch=""
  case "$uname_s" in
    Linux)  os="linux" ;;
    Darwin) os="darwin" ;;
    *) err "unsupported OS: $uname_s (use install.ps1 on Windows)" ;;
  esac
  case "$uname_m" in
    x86_64|amd64) arch="x86_64" ;;
    arm64|aarch64) arch="aarch64" ;;
    *) err "unsupported arch: $uname_m" ;;
  esac

  local target
  case "$os-$arch" in
    darwin-aarch64) target="aarch64-apple-darwin" ;;
    darwin-x86_64)  target="x86_64-apple-darwin" ;;
    linux-x86_64)   target="x86_64-unknown-linux-gnu" ;;
    linux-aarch64)  target="aarch64-unknown-linux-gnu" ;;
  esac

  local url
  if [ "$VERSION" = "latest" ]; then
    url="${REPO_URL}/releases/latest/download/ktlint-rs-${target}.tar.gz"
  else
    url="${REPO_URL}/releases/download/${VERSION}/ktlint-rs-${target}.tar.gz"
  fi
  info "downloading ktlint-rs ${VERSION} for ${target}..."

  local tmp="$(mktemp -d)"
  trap 'rm -rf "${tmp}"' EXIT
  curl -fsSL --retry 3 -o "$tmp/ktlint-rs.tar.gz" "$url" \
    || err "download failed: $url"
  tar -xzf "$tmp/ktlint-rs.tar.gz" -C "$tmp"

  local bin
  [ -f "$tmp/ktlint-rs" ] && bin="$tmp/ktlint-rs"
  [ -z "${bin:-}" ] && err "tarball did not contain ktlint-rs binary"

  mkdir -p "$PREFIX"
  install -m 0755 "$bin" "$PREFIX/ktlint-rs"
  info "installed → $PREFIX/ktlint-rs"
}

# ── cargo fallback ─────────────────────────────────────────────────
try_cargo() {
  if ! command -v cargo >/dev/null 2>&1; then
    return 1
  fi
  local ver="${VERSION}"
  if [ "$ver" = "latest" ]; then ver=""; fi
  info "installing via cargo..."
  cargo install ktlint-rs ${ver:+--version "$ver"} 2>&1 || {
    warn "cargo install failed — falling back to binary"
    return 1
  }
  info "installed via cargo → $(command -v ktlint-rs 2>/dev/null || echo '~/.cargo/bin')"
}

# ── PATH check ─────────────────────────────────────────────────────
check_path() {
  local dir="$(command -v ktlint-rs 2>/dev/null | xargs dirname 2>/dev/null || echo "$PREFIX")"
  if ! echo ":$PATH:" | grep -q ":$dir:"; then
    warn "$dir is not in your PATH. Add it:"
    printf '\033[33m  export PATH="%s:$PATH"\033[0m\n' "$dir"
  fi
}

# ── main ───────────────────────────────────────────────────────────
download_binary
if ! command -v ktlint-rs >/dev/null 2>&1; then
  try_cargo || true
fi
check_path
ktlint-rs --version 2>/dev/null || info "run: ktlint-rs --help"
