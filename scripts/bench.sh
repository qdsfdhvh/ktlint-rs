#!/usr/bin/env bash
# Benchmark ktlint-rs vs ktlint JVM on real-world Kotlin projects.
# Usage: ./scripts/bench.sh [--release]
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"

# ── ktlint-rs binary ──
BUILD_MODE="${1:-debug}"
if [[ "$BUILD_MODE" == "--release" ]]; then
  KTLINT_RS="$REPO_ROOT/target/release/ktlint-rs"
  if [[ ! -x "$KTLINT_RS" ]]; then
    echo "Building ktlint-rs (release)..."
    cargo build --release --manifest-path "$REPO_ROOT/Cargo.toml"
  fi
else
  KTLINT_RS="$REPO_ROOT/target/debug/ktlint-rs"
  if [[ ! -x "$KTLINT_RS" ]]; then
    echo "Building ktlint-rs (debug)..."
    cargo build --manifest-path "$REPO_ROOT/Cargo.toml"
  fi
fi

# ── ktlint JVM (prefer jar, then brew, then download) ──
find_ktlint_cmd() {
  # 1. brew-installed ktlint (recommended)
  if [[ -x /opt/homebrew/bin/ktlint ]]; then
    echo /opt/homebrew/bin/ktlint
    return
  fi
  # 2. local jar download
  local jar
  jar=$(ls -1t "$REPO_ROOT/.ktlint/ktlint-"*.jar 2>/dev/null | head -1 || true)
  if [[ -n "$jar" ]]; then
    echo "java -jar $jar"
    return
  fi
  # 3. download jar
  echo "Downloading ktlint JVM..." >&2
  bash "$REPO_ROOT/scripts/get-ktlint.sh"
  jar=$(ls -1t "$REPO_ROOT/.ktlint/ktlint-"*.jar | head -1)
  echo "java -jar $jar"
}

KTLINT_JVM_CMD=$(find_ktlint_cmd)
run_jvm() { $KTLINT_JVM_CMD "$@"; }
# ── Versions ──
echo "ktlint-rs: $($KTLINT_RS --version 2>&1 | head -1)"
echo "ktlint JVM: $(run_jvm --version 2>&1 | head -1)"
echo ""

# ── Project definitions ──
declare -A DISPLAY_NAMES=()  # optional overrides; falls through to PROJECTS names
declare -A PROJECTS=(
  ["tests/fixtures/demo-gradle"]="demo-gradle"
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
    | awk '{if (NF>1 && $2!="total") sum+=$1} END {print sum+0}'
}

run_tool() {
  local tool="$1"; shift
  if [[ "$tool" == "rs" ]]; then
    "$KTLINT_RS" "$@" 2>&1
  else
    run_jvm "$@" 2>&1
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
per_rule_breakdown() {
  # Extracts rule IDs from violation lines, counts occurrences.
  # Accepts stdin to avoid re-running lint.
  grep -o '(standard:[^)]*)' \
    | tr -d '()' \
    | sort | uniq -c | sort -rn \
    | awk '{printf "%d %s\n", $1, $2}'
}

count_unique_rules() {
  # From raw violation output via stdin, count distinct rule IDs
  grep -o '(standard:[^)]*)' | tr -d '()' | sort -u | wc -l | tr -d ' '
}

count_files_with_violations() {
  # From raw violation output via stdin, count distinct file paths
  grep '\.kt:' | sed 's/:.*//' | sort -u | wc -l | tr -d ' '
}

extract_violations() {
  # From lint output, extract only violation lines
  grep '\.kt:' || true
}

time_run() {
  local tool="$1"; shift
  TIMEFORMAT='%R'
  if [[ "$tool" == "rs" ]]; then
    { time "$KTLINT_RS" "$@" > /dev/null 2>&1; } 2>&1
  else
    { time run_jvm "$@" > /dev/null 2>&1; } 2>&1
  fi
}

exit_code_of() {
  local tool="$1"; shift
  local rc=0
  if [[ "$tool" == "rs" ]]; then
    "$KTLINT_RS" "$@" > /dev/null 2>&1 || rc=$?
  else
    run_jvm "$@" > /dev/null 2>&1 || rc=$?
  fi
  echo ${rc:-0}
}

# ── CSV/TSV output file ──
BENCH_OUT="$REPO_ROOT/bench_results.tsv"
{
  printf "project\tfiles\tlines\trs_violations\tjvm_violations\trs_rules\tjvm_rules\trs_files_hit\tjvm_files_hit\trs_time\tjvm_time\trs_exit\tjvm_exit\n"
} > "$BENCH_OUT"

echo "======= Benchmarks ======="
echo ""

# Temp storage for per-rule data (reuse captured lint output)
RULE_TMPDIR=$(mktemp -d /tmp/ktlint-bench-XXXXXX)
trap 'rm -rf "$RULE_TMPDIR"' EXIT

for fixture_path in "${!PROJECTS[@]}"; do
  name="${DISPLAY_NAMES[$fixture_path]:-${PROJECTS[$fixture_path]}}"
  full_path="$REPO_ROOT/$fixture_path"

  # resolve dirs
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

  # ── Run lint once per tool, capture all output ──
  rs_out=$("$KTLINT_RS" "${dirs[@]}" 2>&1) || true
  jvm_out=$(run_jvm "${dirs[@]}" 2>&1) || true

  # ── Derive metrics ──
  rs_v=$(echo "$rs_out"  | extract_violations | wc -l | tr -d ' ')
  jvm_v=$(echo "$jvm_out" | extract_violations | wc -l | tr -d ' ')
  rs_ur=$(echo "$rs_out" | count_unique_rules)
  jvm_ur=$(echo "$jvm_out" | count_unique_rules)
  rs_fv=$(echo "$rs_out" | count_files_with_violations)
  jvm_fv=$(echo "$jvm_out" | count_files_with_violations)

  # ── Time (separate dry-run for accurate wall-clock) ──
  rs_t=$(time_run rs "${dirs[@]}" 2>/dev/null) || true
  jvm_t=$(time_run jvm "${dirs[@]}" 2>/dev/null) || true

  # ── Exit codes ──
  rs_rc=$(exit_code_of rs "${dirs[@]}" 2>/dev/null)
  jvm_rc=$(exit_code_of jvm "${dirs[@]}" 2>/dev/null)

  # ── Per-rule parity (save to temp for reuse) ──
  safe="${fixture_path//\//_}"
  echo "$rs_out"  | per_rule_breakdown > "$RULE_TMPDIR/${safe}_rs"
  echo "$jvm_out" | per_rule_breakdown > "$RULE_TMPDIR/${safe}_jvm"

  # ── Append TSV row ──
  printf "%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\n" \
    "$name" "$files" "$lines" "$rs_v" "$jvm_v" "$rs_ur" "$jvm_ur" "$rs_fv" "$jvm_fv" "$rs_t" "$jvm_t" "$rs_rc" "$jvm_rc" \
    >> "$BENCH_OUT"

  # ── Print per-project table ──
  printf '━%.0s' {1..42}; echo ""
  printf "  %-30s %'d files, %'d lines\n" "$name" $files $lines
  printf '─%.0s' {1..42}; echo ""
  printf "  %-24s %9s %9s\n" "" "ktlint-rs" "ktlint JVM"
  printf "  %-24s %'9d %'9d\n" "Violations" "$rs_v" "$jvm_v"
  printf "  %-24s %9s %9s\n" "Unique rules" "$rs_ur" "$jvm_ur"
  printf "  %-24s %9s %9s\n" "Files with violations" "$rs_fv" "$jvm_fv"
  printf "  %-24s %9ss %9ss\n" "Time" "$rs_t" "$jvm_t"
  printf "  %-24s %9s %9s\n" "Exit code" "$rs_rc" "$jvm_rc"
  printf '─%.0s' {1..42}; echo ""

  # ── Per-rule breakdown ──
  echo ""
  printf "  %-40s %9s %9s  %s\n" "Rule" "ktlint-rs" "ktlint JVM" ""
  printf "  %-40s %9s %9s\n" "----------------------------------------" "---------" "---------"

  all_rules=$(
    (cat "$RULE_TMPDIR/${safe}_rs" "$RULE_TMPDIR/${safe}_jvm") \
      | awk '{print $2}' | sort -u
  )

  matched=0 total=0
  while IFS= read -r rule; do
    [[ -z "$rule" ]] && continue
    total=$((total + 1))
    rs_count=$(awk -v r="$rule" '$2==r {print $1}' "$RULE_TMPDIR/${safe}_rs")
    jvm_count=$(awk -v r="$rule" '$2==r {print $1}' "$RULE_TMPDIR/${safe}_jvm")
    rs_count=${rs_count:-0}
    jvm_count=${jvm_count:-0}
    if [[ "$rs_count" == "$jvm_count" ]]; then
      flag=" ✓"
      [[ "$rs_count" != "0" ]] && matched=$((matched + 1))
    elif [[ "$rs_count" != "0" && "$jvm_count" == "0" ]]; then
      flag=" ← rs only"
    elif [[ "$rs_count" == "0" && "$jvm_count" != "0" ]]; then
      flag=" → jvm only"
    else
      flag=" ~"
    fi
    printf "  %-40s %9s %9s  %s\n" "$rule" "$rs_count" "$jvm_count" "$flag"
  done <<< "$all_rules"

  if [[ $total -gt 0 ]]; then
    match_pct=$(( matched * 100 / total ))
  else
    match_pct="-"
  fi
  printf "  %-40s %9s %9s\n" "----------------------------------------" "---------" "---------"
  printf "  %-40s %9s\n" "Rule parity (exact match)" "${match_pct}% ($matched/$total)"

  echo ""
  printf '━%.0s' {1..42}; echo ""
  echo ""
done

echo "TSV data written to: $BENCH_OUT"