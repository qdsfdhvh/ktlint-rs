#!/usr/bin/env bash
# Benchmark ktlint-rs vs ktlint JVM on real-world Kotlin projects.
# Usage: ./scripts/bench.sh [--release]
set -euo pipefail

RELEASE=false
if [[ "${1:-}" == "--release" ]]; then
  RELEASE=true
  shift
fi

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
KTLINT_RS="$REPO_ROOT/target/release/ktlint"
KTLINT_JVM="ktlint"

if $RELEASE; then
  echo "Building ktlint-rs (release)..."
  cargo build --release --manifest-path "$REPO_ROOT/Cargo.toml" 2>/dev/null
else
  echo "Building ktlint-rs (debug)..."
  cargo build --manifest-path "$REPO_ROOT/Cargo.toml" 2>/dev/null
  KTLINT_RS="$REPO_ROOT/target/debug/ktlint"
fi

echo "ktlint-rs: $($KTLINT_RS --version 2>&1 | head -1)"
echo "ktlint JVM: $($KTLINT_JVM --version 2>&1 | head -1)"
echo ""

# Benchmark projects (submodule fixture → display name)
declare -A PROJECTS=(
  ["tests/fixtures/nowinandroid"]="nowinandroid"
  ["tests/fixtures/compose-samples"]="compose-samples (6 apps)"
  ["tests/fixtures/okhttp"]="okhttp"
  ["tests/fixtures/androidx"]="androidx (26 modules)"
)

# AndroidX subdirs to benchmark
ANDROIDX_DIRS=(
  activity annotation autofill biometric browser collection concurrent
  datastore documentfile drawerlayout emoji fragment graphics gridlayout
  loader palette preference print savedstate slidingpanelayout startup
  swiperefreshlayout transition vectordrawable viewpager viewpager2
)

# ── Helpers ──

count_files() {
  find "$1" -type f \( -name '*.kt' -o -name '*.kts' \) 2>/dev/null | wc -l | tr -d ' '
}

count_lines() {
  find "$1" -type f \( -name '*.kt' -o -name '*.kts' \) -exec wc -l {} + 2>/dev/null \
    | awk '{if (NF>1 && $2!="total") sum+=$1} END {print sum}'
}

count_violations() {
  local tool="$1"  # ktlint-rs or ktlint-jvm
  local dir="$2"
  if [[ "$tool" == "ktlint-rs" ]]; then
    "$KTLINT_RS" "$dir" 2>&1 | grep -c '\.kt:' || echo 0
  else
    "$KTLINT_JVM" "$dir" 2>&1 | grep -c '\.kt:' || echo 0
  fi
}

time_run() {
  local tool="$1"
  local dirs=("${@:2}")
  if [[ "$tool" == "ktlint-rs" ]]; then
    TIMEFORMAT='%R'; { time "$KTLINT_RS" "${dirs[@]}" > /dev/null 2>&1; } 2>&1
  else
    TIMEFORMAT='%R'; { time "$KTLINT_JVM" "${dirs[@]}" > /dev/null 2>&1; } 2>&1
  fi
}

# ── Table header ──
printf "%-35s %7s %10s %10s %10s %10s %8s %8s\n" \
  "Project" "Files" "Lines" "" "Violations" "" "Time" ""
printf "%-35s %7s %10s %10s %10s %10s %8s %8s\n" \
  "" "" "(rs)" "(JVM)" "(rs)" "(JVM)" "(rs)" "(JVM)"
printf "%s\n" "$(printf '%.0s-' {1..110})"

# ── Data rows ──
for fixture_path in "${!PROJECTS[@]}"; do
  name="${PROJECTS[$fixture_path]}"
  full_path="$REPO_ROOT/$fixture_path"

  # AndroidX special case: only benchmark selected subdirs
  if [[ "$fixture_path" == *"androidx"* ]]; then
    files=0; lines=0
    for d in "${ANDROIDX_DIRS[@]}"; do
      files=$((files + $(count_files "$full_path/$d")))
    done
    lines=$(for d in "${ANDROIDX_DIRS[@]}"; do count_lines "$full_path/$d"; done | awk '{sum+=$1} END {print sum}')

    rs_violations=0; jvm_violations=0
    for d in "${ANDROIDX_DIRS[@]}"; do
      rs_violations=$((rs_violations + $(count_violations "ktlint-rs" "$full_path/$d")))
      jvm_violations=$((jvm_violations + $(count_violations "ktlint-jvm" "$full_path/$d")))
    done

    dirs_for_timing=()
    for d in "${ANDROIDX_DIRS[@]}"; do dirs_for_timing+=("$full_path/$d"); done
    rs_time=$(time_run "ktlint-rs" "${dirs_for_timing[@]}")
    jvm_time=$(time_run "ktlint-jvm" "${dirs_for_timing[@]}")
  else
    files=$(count_files "$full_path")
    lines=$(count_lines "$full_path")
    rs_violations=$(count_violations "ktlint-rs" "$full_path")
    jvm_violations=$(count_violations "ktlint-jvm" "$full_path")
    rs_time=$(time_run "ktlint-rs" "$full_path")
    jvm_time=$(time_run "ktlint-jvm" "$full_path")
  fi

  printf "%-35s %7s %10s %10s %10s %10s %8s %8s\n" \
    "$name" \
    "$files" \
    "$(printf "%'d" $lines)" \
    "$(printf "%'d" $lines)" \
    "$(printf "%'d" $rs_violations)" \
    "$(printf "%'d" $jvm_violations)" \
    "${rs_time}s" \
    "${jvm_time}s"
done

printf "%s\n" "$(printf '%.0s-' {1..110})"
echo "Done. Run with --release for optimized build."
