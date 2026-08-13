#!/usr/bin/env bash
# Format parity measurement: JVM ktlint --format vs ktlint-rs --format on a
# sample of each consumer fixture. Each file is formatted in a temp tree that
# carries the fixture's root .editorconfig (all three fixtures have no
# per-subdir editorconfig), so both tools resolve the same indent_size —
# copying files to a bare tmp dir would silently fall back to JVM's default
# indent (4) and produce false gaps.
#
# Categories:
#   identical   — neither tool changes the file
#   both_same   — both change it to the SAME bytes
#   jvm_only    — JVM changes it, we leave it (format gap: JVM fixes we miss)
#   we_only     — we change it, JVM leaves it (REGRESSION — must stay 0)
#   both_diff   — both change it but to DIFFERENT bytes (implementation gap)
#
# Usage: ./scripts/format-parity.sh <ktor|okhttp|nowinandroid> [sample_size]
set -uo pipefail
PROJ="${1:?usage: format-parity.sh <ktor|okhttp|nowinandroid> [sample_size]}"
SAMPLE="${2:-50}"
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
RS="$ROOT/target/release/ktlint-rs"
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT
FIX="$ROOT/tests/fixtures/$PROJ"
[ -f "$FIX/.editorconfig" ] && cp "$FIX/.editorconfig" "$TMP/.editorconfig"

total=0; identical=0; both_same=0; jvm_only=0; we_only=0; both_diff=0
while IFS= read -r f; do
    total=$((total+1))
    [ "$total" -gt "$SAMPLE" ] && break
    rel="${f#./}"
    rm -rf "$TMP/.cache"
    cp "$FIX/$rel" "$TMP/j.kt"
    cp "$FIX/$rel" "$TMP/r.kt"
    (cd "$TMP" && ktlint --format j.kt >/dev/null 2>&1)
    (cd "$TMP" && "$RS" --format r.kt >/dev/null 2>&1)
    jv=$(cmp -s "$TMP/j.kt" "$FIX/$rel" && echo same || echo diff)
    rv=$(cmp -s "$TMP/r.kt" "$FIX/$rel" && echo same || echo diff)
    jr=$(cmp -s "$TMP/j.kt" "$TMP/r.kt" && echo same || echo diff)
    if [ "$jv" = same ] && [ "$rv" = same ]; then identical=$((identical+1))
    elif [ "$jv" = diff ] && [ "$rv" = diff ] && [ "$jr" = same ]; then both_same=$((both_same+1))
    elif [ "$jv" = same ] && [ "$rv" = diff ]; then we_only=$((we_only+1)); echo "WE_ONLY: $f"
    elif [ "$jv" = diff ] && [ "$rv" = same ]; then jvm_only=$((jvm_only+1))
    else both_diff=$((both_diff+1))
    fi
    rm -f "$TMP/j.kt" "$TMP/r.kt"
done < <(cd "$FIX" && find . -name '*.kt' -not -path '*/build/*' | sort)
consistency=$((identical + both_same))
pct() { python3 -c "import sys; print(f'{round($1/$total*100)}%')"; }
echo "== $PROJ (sample $total): consistent=$(pct $consistency) identical=$identical both_same=$both_same jvm_only=$jvm_only we_only=$we_only both_diff=$both_diff"
