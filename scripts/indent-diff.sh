#!/usr/bin/env bash
# Indent-rule differential: ktlint-rs (probe) vs JVM ktlint 1.8, whole corpus.
# Reports agree / FP (we report, JVM silent) / missed (JVM reports, we don't)
# for standard:indent at identical (file:line:col). Normalizes both outputs
# so message differences don't count as misses.
#
# Usage: ./scripts/indent-diff.sh <fixture-dir>   (run from repo root)
# NOTE: clears .cache in the fixture dir first — cached results would hide
# probe reports (RULES_VERSION in src/cache.rs does not change between builds).
set -uo pipefail
FIXTURE="${1:?usage: indent-diff.sh <fixture-dir>}"
BIN="${BIN:-./target/release/ktlint-rs}"
JVM="${JVM:-ktlint}"
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT/tests/fixtures/$FIXTURE" || exit 1

find . -name ".cache" -type d -exec rm -rf {} + 2>/dev/null
KTLINT_RS_INDENT_PROBE=1 "$ROOT/$BIN" --relative . 2>/dev/null \
  | grep "standard:indent" | python3 "$ROOT/scripts/normalize_lint.py" > /tmp/rs_lint.txt
ktlint --relative $(find . -name '*.kt') 2>/dev/null \
  | grep "standard:indent" | python3 "$ROOT/scripts/normalize_lint.py" > /tmp/jvm_lint.txt

echo "jvm: $(wc -l < /tmp/jvm_lint.txt)  rs: $(wc -l < /tmp/rs_lint.txt)"
echo "  agree(双方都报):    $(comm -12 /tmp/jvm_lint.txt /tmp/rs_lint.txt | wc -l)"
echo "  FP(我们报 JVM不报):  $(comm -13 /tmp/jvm_lint.txt /tmp/rs_lint.txt | wc -l)"
echo "  missed(JVM报我们不): $(comm -23 /tmp/jvm_lint.txt /tmp/rs_lint.txt | wc -l)"
echo "--- FP 前 5 ---"
comm -13 /tmp/jvm_lint.txt /tmp/rs_lint.txt | head -5
echo "--- missed 前 5 ---"
comm -23 /tmp/jvm_lint.txt /tmp/rs_lint.txt | head -5
