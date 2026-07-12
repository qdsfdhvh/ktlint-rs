#!/usr/bin/env bash
# Setup test fixtures for real-world Kotlin project benchmarks.
# Shallow clones popular open-source Kotlin repos into tests/fixtures/.
#
# Usage:
#   ./scripts/setup-fixtures.sh              # clone all
#   ./scripts/setup-fixtures.sh nowinandroid # clone one
#   ./scripts/setup-fixtures.sh --list       # list available
#   ./scripts/setup-fixtures.sh --clean      # remove all fixtures
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
FIXTURES_DIR="$REPO_ROOT/tests/fixtures"

# ── Registry: name → repo URL, options ──
# Key: fixture directory name
# Value: "url [sparse_paths...]"
declare -A FIXTURES=(
  ["nowinandroid"]="https://github.com/android/nowinandroid.git"
  ["compose-samples"]="https://github.com/android/compose-samples.git"
  ["okhttp"]="https://github.com/square/okhttp.git"
  ["androidx"]="https://github.com/androidx/androidx.git activity appcompat compose fragment lifecycle navigation paging work room3 datastore"
)

# ── Clone helper ──

clone_fixture() {
  local name="$1"
  local entry="${FIXTURES[$name]}"
  if [[ -z "$entry" ]]; then
    echo "Unknown fixture: $name. Run --list to see available."
    return 1
  fi

  local url=$(echo "$entry" | awk '{print $1}')
  local sparse=$(echo "$entry" | awk '{for(i=2;i<=NF;i++) printf "%s ", $i}' | sed 's/ $//')

  local target="$FIXTURES_DIR/$name"
  if [[ -d "$target/.git" ]]; then
    echo "[$name] already exists, updating..."
    git -C "$target" pull --depth 1 --ff-only 2>/dev/null || true
    return
  fi

  echo "[$name] cloning $url..."
  rm -rf "$target"

  if [[ -n "$sparse" ]]; then
    # Sparse checkout for monorepos (e.g. AndroidX):
    # only checkout specific subdirectories to save space.
    mkdir -p "$target"
    git -C "$target" init -q
    git -C "$target" remote add origin "$url"
    git -C "$target" config core.sparseCheckout true
    for path in $sparse; do
      echo "$path/*" >> "$target/.git/info/sparse-checkout"
    done
    # Also include top-level build files for Gradle
    echo "settings.gradle" >> "$target/.git/info/sparse-checkout"
    echo "build.gradle" >> "$target/.git/info/sparse-checkout"
    echo "gradle.properties" >> "$target/.git/info/sparse-checkout"
    echo "gradle/" >> "$target/.git/info/sparse-checkout"
    git -C "$target" fetch --depth 1 origin main 2>/dev/null \
      || git -C "$target" fetch --depth 1 origin master
    git -C "$target" checkout FETCH_HEAD
  else
    git clone --depth 1 --single-branch "$url" "$target"
  fi
  echo "[$name] done."
}

# ── Main ──

case "${1:-}" in
  --list)
    echo "Available fixtures:"
    for name in "${!FIXTURES[@]}"; do
      printf "  %-20s %s\n" "$name" "${FIXTURES[$name]}"
    done
    ;;
  --clean)
    echo "Removing all fixtures..."
    for name in "${!FIXTURES[@]}"; do
      rm -rf "$FIXTURES_DIR/$name"
    done
    echo "Done."
    ;;
  "")
    echo "Setting up all fixtures..."
    mkdir -p "$FIXTURES_DIR"
    for name in "${!FIXTURES[@]}"; do
      clone_fixture "$name"
    done
    echo ""
    echo "All fixtures ready. Run: ./scripts/bench.sh --release"
    ;;
  *)
    mkdir -p "$FIXTURES_DIR"
    clone_fixture "$1"
    ;;
esac
