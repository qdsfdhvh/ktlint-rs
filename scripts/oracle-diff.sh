#!/usr/bin/env bash
# Oracle differential for the regression case corpus.
#
# Runs ktlint-rs against every fixture in tests/fixtures/oracle-cases/ and
# compares the violations of the rule under test with JVM ktlint 1.8.0
# (line:col + message, exact match). Every case added to the corpus must pass
# here before a rule change is merged — this is the net that catches the
# "fixed one case, broke another" class of regression.
#
# Usage:
#   KTLINT_RS=./target/release/ktlint-rs ./scripts/oracle-diff.sh [--rule RULE] [--only DIR]
#
# --rule  compare only this rule id (default: the corpus dir's theme rule)
# --only  restrict to one corpus dir (issue176-ktlint-official, ...)
#
# Corpus dirs are named after the issue/feature they regress; the default
# compared rule follows the dir name (issue176*/for-header* -> standard:indent,
# issue177* -> standard:class-signature, issue178* -> standard:curly-spacing).
#
# If the JVM ktlint binary is missing the gate is skipped (same policy as the
# spotless differential in CI).

set -uo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
KTLINT_RS="${KTLINT_RS:-$ROOT/target/release/ktlint-rs}"
KTLINT_JVM="${KTLINT_JVM:-ktlint}"
CORPUS="$ROOT/tests/fixtures/oracle-cases"
RULE=""
ONLY=""

while [[ $# -gt 0 ]]; do
    case "$1" in
        --rule) RULE="${2:?--rule requires a rule id}"; shift ;;
        --only) ONLY="${2:?--only requires a dir name}"; shift ;;
        *) echo "usage: $0 [--rule RULE] [--only DIR]" >&2; exit 2 ;;
    esac
    shift
done

if ! command -v "$KTLINT_JVM" >/dev/null 2>&1; then
    echo "oracle ktlint not found (\$KTLINT_JVM), skipping oracle-diff gate"
    exit 0
fi

[ -x "$KTLINT_RS" ] || { echo "ktlint-rs binary not found: $KTLINT_RS" >&2; exit 1; }

fail=0
total=0
for dir in "$CORPUS"/*/; do
    name="$(basename "$dir")"
    [ -n "$ONLY" ] && [ "$name" != "$ONLY" ] && continue
    # Theme rule for the corpus dir; --rule overrides.
    local_rule="$RULE"
    if [ -z "$local_rule" ]; then
        case "$name" in
            issue177*) local_rule="standard:class-signature" ;;
            issue178*) local_rule="standard:curly-spacing" ;;
            *) local_rule="standard:indent" ;;
        esac
    fi
    for f in "$dir"*.kt; do
        [ -e "$f" ] || continue
        total=$((total + 1))
        base="$(basename "$f")"
        # oracle output: file:line:col: message (standard:rule)
        oracle=$("$KTLINT_JVM" --relative "$f" 2>/dev/null \
            | grep "($local_rule)" \
            | sed 's/^[^:]*\.kt:\([0-9]*\):\([0-9]*\): \(.*\) (standard:[a-z-]*)$/\1:\2 \3/')
        # ktlint-rs output: file:line:col (standard:rule) message
        rs=$("$KTLINT_RS" --ruleset ktlint --relative "$f" 2>&1 \
            | grep "($local_rule)" \
            | sed 's/^[^:]*\.kt:\([0-9]*\):\([0-9]*\) (standard:[a-z-]*) \(.*\)$/\1:\2 \3/')
        if [ "$oracle" == "$rs" ]; then
            echo "OK   $name/$base"
        else
            echo "DIFF $name/$base"
            echo "  oracle: $(echo "$oracle" | tr '\n' ';')"
            echo "  rs:     $(echo "$rs" | tr '\n' ';')"
            fail=1
        fi
    done
done

echo "----"
echo "oracle-diff: $total cases, rule=${RULE:-per-dir}, $([ $fail -eq 0 ] && echo 'ALL MATCH' || echo 'DIFFS FOUND')"
exit $fail
