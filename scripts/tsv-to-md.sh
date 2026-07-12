#!/usr/bin/env bash
# Convert bench_results.tsv to per-project markdown tables.
# Usage: ./scripts/tsv-to-md.sh [tsv_file]  (default: bench_results.tsv)
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
TSV="${1:-$REPO_ROOT/bench_results.tsv}"

if [[ ! -f "$TSV" ]]; then
  echo "Error: $TSV not found. Run ./scripts/bench.sh first." >&2
  exit 1
fi

# ── Per-project markdown table ──
# Outputs:
#   ### nowinandroid
#   350 files, 62,042 lines
#
#   | Metric | ktlint-rs | ktlint JVM |
#   |--------|-----------|------------|
#   | Violations | 4,419 | 1,038 |
#   | ... | | |

print_project() {
  local name="$1" files="$2" lines="$3" rs_v="$4" jvm_v="$5"
  local rs_ur="$6" jvm_ur="$7" rs_fv="$8" jvm_fv="$9"
  local rs_t="${10}" jvm_t="${11}" rs_exit="${12}" jvm_exit="${13}"

  echo "### $name"
  echo "$files files, $(printf "%'d" $lines) lines"
  echo ""
  echo "| Metric | ktlint-rs | ktlint JVM |"
  echo "|--------|-----------|------------|"
  echo "| Violations | $(printf "%'d" $rs_v) | $(printf "%'d" $jvm_v) |"
  echo "| Unique rules | $rs_ur | $jvm_ur |"
  echo "| Files with violations | $rs_fv | $jvm_fv |"
  echo "| Time | ${rs_t}s | ${jvm_t}s |"
  local exit_str
  if [[ "$rs_exit" == "$jvm_exit" ]]; then
    exit_str="✓ $rs_exit"
  else
    exit_str="$rs_exit / $jvm_exit"
  fi
  echo "| Exit code | $exit_str |"
  echo ""
}

# ── Overall speedup table ──
echo "## Benchmark Results"
echo ""
echo "| Project | Files | Lines | Violations | Rules | FilesHit | Time | Exit | Speedup |"
echo "|---------|-------|-------|------------|-------|----------|------|------|---------|"

{
  header=true
  while IFS=$'\t' read -r project files lines rs_v jvm_v rs_ur jvm_ur rs_fv jvm_fv rs_t jvm_t rs_exit jvm_exit; do
    $header && { header=false; continue; }
    speedup=$(awk "BEGIN {printf \"%.1fx\", $jvm_t / $rs_t}")
    exit_ok="✓" && [[ "$rs_exit" != "$jvm_exit" ]] && exit_ok="✗"

    echo "| **$project** | $files | $(printf "%'d" $lines) | $(printf "%'d / %'d" $rs_v $jvm_v) | $rs_ur / $jvm_ur | $rs_fv / $jvm_fv | ${rs_t}s / ${jvm_t}s | $exit_ok | $speedup |"
  done
} < "$TSV"

echo ""

# ── Per-project breakdowns ──
{
  header=true
  while IFS=$'\t' read -r project files lines rs_v jvm_v rs_ur jvm_ur rs_fv jvm_fv rs_t jvm_t rs_exit jvm_exit; do
    $header && { header=false; continue; }
    print_project "$project" "$files" "$lines" "$rs_v" "$jvm_v" "$rs_ur" "$jvm_ur" "$rs_fv" "$jvm_fv" "$rs_t" "$jvm_t" "$rs_exit" "$jvm_exit"
  done
} < "$TSV"
