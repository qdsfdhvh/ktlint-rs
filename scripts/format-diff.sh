#!/usr/bin/env bash
# Format differential: --format must be a no-op on every file the JVM ktlint
# 1.8.0 leaves untouched (the whole oracle-cases corpus plus a sample of the
# consumer fixtures). A file that ktlint formats is expected to be formatted
# by us too, but one ktlint leaves alone must come back byte-identical — a
# rewrite there is the issue-#189 class of regression.
#
# Usage: KTLINT_RS=./target/release/ktlint-rs ./scripts/format-diff.sh [--all]
#   --all  also sweep the full consumer fixture trees (slow, JVM ktlint per file)

set -uo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
KTLINT_RS="${KTLINT_RS:-$ROOT/target/release/ktlint-rs}"
KTLINT_JVM="${KTLINT_JVM:-ktlint}"
FILES=()

if ! command -v "$KTLINT_JVM" >/dev/null 2>&1; then
    echo "oracle ktlint not found (\$KTLINT_JVM), skipping format-diff gate"
    exit 0
fi
[ -x "$KTLINT_RS" ] || { echo "ktlint-rs binary not found: $KTLINT_RS" >&2; exit 1; }

# Corpus: every oracle-cases .kt
for f in "$ROOT"/tests/fixtures/oracle-cases/*/*.kt; do
    [ -e "$f" ] && FILES+=("$f")
done
if [[ "${1:-}" == "--all" ]]; then
    for proj in ktor nowinandroid okhttp; do
        while IFS= read -r f; do FILES+=("$f"); done \
            < <(find "$ROOT/tests/fixtures/$proj" -name "*.kt")
    done
fi

# Phase 1: find files our --format rewrites (cheap, no JVM).
rewritten=()
tmpd="$(mktemp -d)"
for f in "${FILES[@]}"; do
    cp "$f" "$tmpd/x.kt"
    if "$KTLINT_RS" --format "$tmpd/x.kt" >/dev/null 2>&1         && ! diff -q "$tmpd/x.kt" "$f" >/dev/null 2>&1; then
        rewritten+=("$f")
    fi
done
# Phase 2: a rewrite is a regression only if the oracle leaves the file alone.
fail=0
for f in "${rewritten[@]}"; do
    cp "$f" "$tmpd/o.kt"
    if "$KTLINT_JVM" --format "$tmpd/o.kt" >/dev/null 2>&1         && diff -q "$tmpd/o.kt" "$f" >/dev/null 2>&1; then
        echo "FORMAT-DIFF: ${f#$ROOT/}"
        cp "$f" "$tmpd/r.kt"
        "$KTLINT_RS" --format "$tmpd/r.kt" >/dev/null 2>&1
        diff "$f" "$tmpd/r.kt" | head -8
        fail=1
    fi
done
rm -rf "$tmpd"
total="${#FILES[@]}"
echo "----"
echo "format-diff: $total files, ${#rewritten[@]} rewritten by us, $([ $fail -eq 0 ] && echo 'NO REGRESSIONS' || echo 'REWRITES THE ORACLE WOULD NOT MAKE')"
exit $fail
