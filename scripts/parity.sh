#!/bin/bash
# Parity benchmark — runs ktlint-rs against all fixtures and reports violations.
# Usage: ./scripts/parity.sh

set -euo pipefail
BIN="${BIN:-./target/release/ktlint-rs}"
FIXTURES="tests/fixtures"

echo "=== ktlint-rs Parity Benchmark ==="
echo "Binary: $BIN"
echo ""

for dir in nowinandroid okhttp; do
    if [ ! -d "$FIXTURES/$dir" ]; then
        echo "  $dir: SKIP (fixture not found)"
        continue
    fi
    for ruleset in ktlint detekt; do
        echo "--- $dir ($ruleset) ---"
        start=$(date +%s)
        output=$($BIN --ruleset "$ruleset" "$FIXTURES/$dir" 2>&1) || true
        end=$(date +%s)
        violations=$(echo "$output" | wc -l | tr -d ' ')
        rules=$(echo "$output" | grep "Summary error count" -A200 | grep "detekt:\|standard:" | wc -l | tr -d ' ')
        echo "  Time: $((end - start))s"
        echo "  Violations: $violations lines"
        echo "  Rules triggered: $rules"
        echo "  Top 5:"
        echo "$output" | grep "Summary error count" -A200 | grep "detekt:\|standard:" | head -5
        echo ""
    done
done

echo "=== Done ==="
