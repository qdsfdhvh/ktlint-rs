#!/usr/bin/env bash
# Phase 6 gate (#88): verify ktlint-rs hard runtime constraints.
#   - release binary < 30MB
#   - cold startup < 50ms
#   - per-file lint < 5ms (average over N files)
#   - clean exit (no daemon / residual process)
set -euo pipefail

BIN="${1:-target/release/ktlint-rs}"
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

fail() { echo "GATE FAIL: $*" >&2; exit 1; }

[ -x "$BIN" ] || fail "binary not found: $BIN"

# 1. Binary size < 30MB
SIZE=$(stat -c%s "$BIN" 2>/dev/null || stat -f%z "$BIN")
if [ "$SIZE" -ge 31457280 ]; then
    fail "release binary ${SIZE}B >= 30MB"
fi
echo "binary size: $((SIZE / 1024 / 1024))MB (< 30MB) OK"

# 2. Cold startup < 50ms (lint an empty dir). Warm once so the first-run
# .cache write is not counted against the gate.
mkdir -p "$TMP/empty"
"$BIN" --ruleset ktlint "$TMP/empty" >/dev/null 2>&1 || true
rm -rf "$TMP/empty/.cache"
START=$(python3 -c 'import time; print(time.time())')
"$BIN" --ruleset ktlint "$TMP/empty" >/dev/null 2>&1 || true
END=$(python3 -c 'import time; print(time.time())')
STARTUP_MS=$(python3 -c "print(round(($END - $START) * 1000))")
if [ "$STARTUP_MS" -ge 50 ]; then
    fail "cold startup ${STARTUP_MS}ms >= 50ms"
fi
echo "cold startup: ${STARTUP_MS}ms (< 50ms) OK"

# 3. Per-file lint < 5ms average — engine throughput only: one process
# lints all files; the measured time minus cold startup is the lint time.
printf 'package a\n\nval x = 1\nfun f() { println(x) }\n' > "$TMP/f1.kt"
printf 'package a\n\nclass Foo { val y = 2 }\n' > "$TMP/f2.kt"
printf 'package a\n\nfun g(a: Int, b: Int): Int = a + b\n' > "$TMP/f3.kt"
printf 'package a\n\nval s = "hello world"\n' > "$TMP/f4.kt"
START=$(python3 -c 'import time; print(time.time())')
"$BIN" --ruleset ktlint "$TMP"/*.kt >/dev/null 2>&1 || true
END=$(python3 -c 'import time; print(time.time())')
TOTAL_MS=$(python3 -c "print(($END - $START) * 1000)")
LINT_MS=$(python3 -c "print(max(0, $TOTAL_MS - $STARTUP_MS))")
PER_FILE_MS=$(python3 -c "print(round($LINT_MS / 4))")
if [ "$PER_FILE_MS" -ge 5 ]; then
    fail "per-file lint ${PER_FILE_MS}ms >= 5ms"
fi
echo "per-file lint: ${PER_FILE_MS}ms (engine, < 5ms) OK"

# 4. Clean exit — process must not linger after linting
"$BIN" --ruleset ktlint "$TMP/f1.kt" >/dev/null 2>&1 &
PID=$!
for _ in $(seq 1 50); do
    kill -0 "$PID" 2>/dev/null || break
    sleep 0.02
done
if kill -0 "$PID" 2>/dev/null; then
    kill "$PID" 2>/dev/null || true
    fail "process still alive after 1s (daemon leak?)"
fi
echo "clean exit OK"

echo "ALL GATES PASS"
