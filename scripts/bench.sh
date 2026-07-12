#!/usr/bin/env bash
# Benchmark ktlint-rs vs ktlint JVM on real-world Kotlin projects.
# Usage: ./scripts/bench.sh [--release]
set -euo pipefail

# The homebrew rustup proxy loses argv[0]; use the real binary.
RUSTUP_BIN="/opt/homebrew/Cellar/rustup/1.29.0_2/libexec/bin/rustup"
RUSTUP_ENV="RUSTUP_OVERRIDE_UNIX_FALLBACK_SETTINGS=/opt/homebrew/etc/rustup/settings.toml"

if $RELEASE; then
  echo "Building ktlint-rs (release)..."
  env $RUSTUP_ENV "$RUSTUP_BIN" run stable cargo build --release --manifest-path "$REPO_ROOT/Cargo.toml" 2>/dev/null
else
  echo "Building ktlint-rs (debug)..."
  env $RUSTUP_ENV "$RUSTUP_BIN" run stable cargo build --manifest-path "$REPO_ROOT/Cargo.toml" 2>/dev/null
  KTLINT_RS="$REPO_ROOT/target/debug/ktlint-rs"
fi

echo "ktlint-rs: $($KTLINT_RS --version 2>&1 | head -1)"
echo "ktlint JVM: $($KTLINT_JVM --version 2>&1 | head -1)"
echo ""

# ── Project definitions ──
declare -A PROJECTS=(
  ["tests/fixtures/nowinandroid"]="nowinandroid"
  ["tests/fixtures/compose-samples"]="compose-samples (6 apps)"
  ["tests/fixtures/okhttp"]="okhttp"
  ["tests/fixtures/androidx"]="androidx (26 modules)"
)

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

run_tool() {
  local tool="$1"; shift
  if [[ "$tool" == "rs" ]]; then
    "$KTLINT_RS" "$@" 2>&1
  else
    "$KTLINT_JVM" "$@" 2>&1
  fi
}

output_violations() {
  local tool="$1"; shift
  run_tool "$tool" "$@" | grep '\.kt:' || true
}

count_violations() {
  output_violations "$@" | wc -l | tr -d ' '
}

per_rule_breakdown() {
  # Extracts rule IDs from violation lines, counts occurrences.
  local tool="$1"; shift
  run_tool "$tool" "$@" \
    | grep -oP '\(standard:[^)]+\)' \
    | tr -d '()' \
    | sort | uniq -c | sort -rn \
    | awk '{printf "%d %s\n", $1, $2}'
}

unique_rule_ids() {
  per_rule_breakdown "$@" | awk '{print $2}' | sort -u
}

unique_files() {
  output_violations "$@" \
    | sed 's/.*fixtures\/[^/]*\///' \
    | sed 's/:.*//' \
    | sort -u
}

time_run() {
  local tool="$1"; shift
  TIMEFORMAT='%R'
  if [[ "$tool" == "rs" ]]; then
    { time "$KTLINT_RS" "$@" > /dev/null 2>&1; } 2>&1
  else
    { time "$KTLINT_JVM" "$@" > /dev/null 2>&1; } 2>&1
  fi
}

exit_code_of() {
  local tool="$1"; shift
  if [[ "$tool" == "rs" ]]; then
    "$KTLINT_RS" "$@" > /dev/null 2>&1; echo $?
  else
    "$KTLINT_JVM" "$@" > /dev/null 2>&1; echo $?
  fi
}

# ── Run benchmarks ──

echo "======= Summary Table ======="
echo ""
printf "%-35s %7s %10s %10s %10s %10s %8s %8s\n" \
  "Project" "Files" "Lines" "" "Violations" "" "Time" ""
printf "%-35s %7s %10s %10s %10s %10s %8s %8s\n" \
  "" "" "(rs)" "(JVM)" "(rs)" "(JVM)" "(rs)" "(JVM)"
printf "%s\n" "$(printf '%.0s-' {1..110})"

for fixture_path in "${!PROJECTS[@]}"; do
  name="${PROJECTS[$fixture_path]}"
  full_path="$REPO_ROOT/$fixture_path"

  if [[ "$fixture_path" == *"androidx"* ]]; then
    files=0; lines=0
    for d in "${ANDROIDX_DIRS[@]}"; do
      files=$((files + $(count_files "$full_path/$d")))
      lines=$((lines + $(count_lines "$full_path/$d")))
    done
    dirs=(); for d in "${ANDROIDX_DIRS[@]}"; do dirs+=("$full_path/$d"); done
  else
    files=$(count_files "$full_path")
    lines=$(count_lines "$full_path")
    dirs=("$full_path")
  fi

  rs_v=$(count_violations rs "${dirs[@]}")
  jvm_v=$(count_violations jvm "${dirs[@]}")
  rs_t=$(time_run rs "${dirs[@]}")
  jvm_t=$(time_run jvm "${dirs[@]}")

  printf "%-35s %7s %10s %10s %10s %10s %8s %8s\n" \
    "$name" "$files" \
    "$(printf "%'d" $lines)" "$(printf "%'d" $lines)" \
    "$(printf "%'d" $rs_v)" "$(printf "%'d" $jvm_v)" \
    "${rs_t}s" "${jvm_t}s"
done
printf "%s\n" "$(printf '%.0s-' {1..110})"
echo ""

# ── Per-rule breakdown ──

for fixture_path in "${!PROJECTS[@]}"; do
  name="${PROJECTS[$fixture_path]}"
  full_path="$REPO_ROOT/$fixture_path"

  if [[ "$fixture_path" == *"androidx"* ]]; then
    dirs=(); for d in "${ANDROIDX_DIRS[@]}"; do dirs+=("$full_path/$d"); done
  else
    dirs=("$full_path")
  fi

  echo "======= $name — Per-Rule Breakdown ======="
  echo ""
  printf "%-45s %8s %8s\n" "Rule" "ktlint-rs" "ktlint JVM"
  printf "%s\n" "$(printf '%.0s-' {1..62})"

  # Collect rule counts from both tools
  declare -A rs_counts=()
  declare -A jvm_counts=()
  while read -r count rule; do rs_counts["$rule"]=$count; done < <(per_rule_breakdown rs "${dirs[@]}")
  while read -r count rule; do jvm_counts["$rule"]=$count; done < <(per_rule_breakdown jvm "${dirs[@]}")

  # All unique rules across both tools
  all_rules=$( (unique_rule_ids rs "${dirs[@]}"; unique_rule_ids jvm "${dirs[@]}") | sort -u)

  matched=0; missed=0; rs_only=0; jvm_only=0
  while IFS= read -r rule; do
    rs_n="${rs_counts[$rule]:-0}"
    jvm_n="${jvm_counts[$rule]:-0}"
    if [[ "$rs_n" -eq 0 && "$jvm_n" -eq 0 ]]; then continue; fi

    # Mark match/miss
    if [[ "$rs_n" -gt 0 && "$jvm_n" -gt 0 && "$rs_n" -eq "$jvm_n" ]]; then
      status="✓"
      matched=$((matched + 1))
    elif [[ "$rs_n" -gt 0 && "$jvm_n" -gt 0 ]]; then
      status="~"
      missed=$((missed + 1))
    elif [[ "$rs_n" -gt 0 ]]; then
      status="rs"
      rs_only=$((rs_only + 1))
    else
      status="jvm"
      jvm_only=$((jvm_only + 1))
    fi
    printf "%-45s %8s %8s  [%s]\n" "$rule" "$rs_n" "$jvm_n" "$status"
  done <<< "$all_rules"

  total_rules=$((matched + missed + rs_only + jvm_only))
  printf "%s\n" "$(printf '%.0s-' {1..62})"
  printf "Rules total: %d  ✓ matched: %d  ~ diff: %d  rs-only: %d  jvm-only: %d\n" \
    "$total_rules" "$matched" "$missed" "$rs_only" "$jvm_only"
  echo ""
done

# ── Exit codes ──

echo "======= Exit Codes ======="
echo ""
for fixture_path in "${!PROJECTS[@]}"; do
  name="${PROJECTS[$fixture_path]}"
  full_path="$REPO_ROOT/$fixture_path"
  if [[ "$fixture_path" == *"androidx"* ]]; then
    dirs=(); for d in "${ANDROIDX_DIRS[@]}"; do dirs+=("$full_path/$d"); done
  else
    dirs=("$full_path")
  fi
  rs_ec=$(exit_code_of rs "${dirs[@]}")
  jvm_ec=$(exit_code_of jvm "${dirs[@]}")
  match="✓"
  if [[ "$rs_ec" != "$jvm_ec" ]]; then match="✗"; fi
  printf "%-35s  rs=%s  jvm=%s  %s\n" "$name" "$rs_ec" "$jvm_ec" "$match"
done

echo ""
echo "Done. Run with --release for optimized build."
