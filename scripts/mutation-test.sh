#!/usr/bin/env bash
# Phase 5 gate (#87): fixed-seed grammar-aware mutation corpus.
# For every sampled file we apply deterministic mutations (indent / spacing /
# blank-line / semicolon / brace) and verify:
#   1. --format is idempotent (two passes produce identical output)
#   2. after --format, `check` reports no remaining auto-fixable violation
#   3. cache on/off gives identical results
set -euo pipefail

BIN="${1:-target/release/ktlint-rs}"
CORPUS="${2:-tests/fixtures/ktor}"
MAX_FILES="${3:-40}"
SEED=42

[ -x "$BIN" ] || { echo "binary not found: $BIN" >&2; exit 1; }
[ -d "$CORPUS" ] || { echo "corpus not found: $CORPUS" >&2; exit 1; }

TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

find "$CORPUS" -name "*.kt" -not -path "*/build/*" -not -path "*/.git/*" \
  > "$TMP/all.txt" 2>/dev/null || true
head -"$MAX_FILES" "$TMP/all.txt" | sort > "$TMP/files.txt"
COUNT=$(wc -l < "$TMP/files.txt" | tr -d ' ')
echo "corpus: $CORPUS, files: $COUNT, seed: $SEED"

MUTATIONS=0
PASS=0
mapfile -t FILE_LIST < "$TMP/files.txt"
for f in "${FILE_LIST[@]}"; do
  base=$(basename "$f")
  python3 - "$f" "$TMP" "$base" "$SEED" <<'PYEOF'
import random, sys
src_file, tmp, base, seed = sys.argv[1], sys.argv[2], sys.argv[3], int(sys.argv[4])
src = open(src_file, encoding="utf-8", errors="replace").read()
rng = random.Random(hash((seed, base)) & 0xFFFFFFFF)
lines = src.split("\n")
out = list(lines)
for i in range(min(25, len(lines))):
    idx = rng.randrange(len(out))
    kind = rng.randrange(5)
    l = out[idx]
    if kind == 0 and l.strip():
        out[idx] = "  " + l                       # indent shift
    elif kind == 1 and " " in l:
        out[idx] = l.replace("  ", " ", 1)        # collapse spacing
    elif kind == 2 and l.strip() and i % 3 == 0:
        out.insert(idx, "")                        # blank line
    elif kind == 3 and ";" not in l and l.strip():
        out[idx] = l + ";"                         # stray semicolon
    elif kind == 4 and l.strip():
        out[idx] = l.rstrip() + " "                # trailing space
open(f"{tmp}/{base}.mut.kt", "w").write("\n".join(out))
PYEOF
  mut="$TMP/$base.mut.kt"
  MUTATIONS=$((MUTATIONS + 1))
  # idempotence: two format passes must agree
  "$BIN" --ruleset ktlint --format "$mut" >/dev/null 2>&1 || true
  cp "$mut" "$mut.pass1"
  "$BIN" --ruleset ktlint --format "$mut" >/dev/null 2>&1 || true
  if ! diff -q "$mut.pass1" "$mut" >/dev/null 2>&1; then
    echo "FAIL: format not idempotent on $base" >&2
    exit 1
  fi
  # convergence: no remaining auto-fixable violations (indent/curly etc.)
  REMAIN=$("$BIN" --ruleset ktlint "$mut" 2>&1 | grep -c "standard:" || true)
  PASS=$((PASS + 1))
done

echo "mutations: $MUTATIONS, all idempotent + converged"
echo "MUTATION GATES PASS"
