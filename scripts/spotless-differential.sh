#!/usr/bin/env bash
# Differential harness for Kataris Spotless 8.8.0 + ktlint 1.8.0.
# Usage:
#   GRADLE=/path/to/gradlew KTLINT_RS=./target/release/ktlint-rs \
#     ./scripts/spotless-differential.sh [--offline] [--expect-mismatch] [--inject-mismatch KIND] [--artifacts DIR]

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
ORACLE_SOURCE="$ROOT/tests/oracle/spotless-8.8.0-ktlint-1.8.0"
HELPER="$ROOT/scripts/spotless_diff.py"
GRADLE="${GRADLE:-gradle}"
KTLINT_RS="${KTLINT_RS:-$ROOT/target/release/ktlint-rs}"
PYTHON="${PYTHON:-python3}"
ARTIFACTS_DIR="$ROOT/target/spotless-diff-artifacts"
EXPECT_MISMATCH=false
OFFLINE=false
INJECT_MISMATCH=""

while [[ $# -gt 0 ]]; do
    case "$1" in
        --offline)
            OFFLINE=true
            ;;
        --expect-mismatch)
            EXPECT_MISMATCH=true
            ;;
        --inject-mismatch)
            shift
            INJECT_MISMATCH="${1:?--inject-mismatch requires discovery, config, diagnostics, format, or idempotence}"
            case "$INJECT_MISMATCH" in
                discovery|config|diagnostics|format|idempotence) ;;
                *) echo "invalid mismatch kind: $INJECT_MISMATCH" >&2; exit 2 ;;
            esac
            EXPECT_MISMATCH=true
            ;;
        --artifacts)
            shift
            ARTIFACTS_DIR="${1:?--artifacts requires a directory}"
            ;;
        *)
            echo "usage: $0 [--offline] [--expect-mismatch] [--inject-mismatch KIND] [--artifacts DIR]" >&2
            exit 2
            ;;
    esac
    shift
done

if [[ ! -x "$KTLINT_RS" ]]; then
    echo "ktlint-rs binary not found or not executable: $KTLINT_RS" >&2
    exit 2
fi
if ! command -v "$PYTHON" >/dev/null 2>&1; then
    echo "Python executable not found: $PYTHON" >&2
    exit 2
fi
if [[ "$OFFLINE" == false ]] && [[ ! -x "$GRADLE" ]] && ! command -v "$GRADLE" >/dev/null 2>&1; then
    echo "Gradle executable not found: $GRADLE" >&2
    exit 2
fi

TMP_ROOT="$(mktemp -d "${TMPDIR:-/tmp}/ktlint-rs-spotless-diff.XXXXXX")"
trap 'rm -rf "$TMP_ROOT"' EXIT

cp -R "$ORACLE_SOURCE" "$TMP_ROOT/input"
cp -R "$ORACLE_SOURCE" "$TMP_ROOT/oracle"
cp -R "$ORACLE_SOURCE" "$TMP_ROOT/actual"
rm -rf \
    "$TMP_ROOT/input/build" "$TMP_ROOT/input/.gradle" \
    "$TMP_ROOT/oracle/build" "$TMP_ROOT/oracle/.gradle" \
    "$TMP_ROOT/actual/build" "$TMP_ROOT/actual/.gradle"
mkdir -p "$TMP_ROOT/results"

if [[ "$EXPECT_MISMATCH" == true && -z "$INJECT_MISMATCH" ]]; then
    for copy in input oracle actual; do
        cat >"$TMP_ROOT/$copy/src/main/kotlin/oracle/ExpectedMismatch.kt" <<'KOTLIN'
package oracle

fun dirty( value:Int )=value+1
KOTLIN
    done
fi
if [[ "$INJECT_MISMATCH" == "discovery" ]]; then
    cat >"$TMP_ROOT/actual/src/main/kotlin/oracle/ActualOnly.kt" <<'KOTLIN'
package oracle

val actualOnly = true
KOTLIN
elif [[ "$INJECT_MISMATCH" == "config" ]]; then
    "$PYTHON" "$HELPER" mutate-config "$TMP_ROOT/actual/effective-config.json"
fi

EXPECTED_DISCOVERY="$TMP_ROOT/results/oracle-discovery.json"
ACTUAL_DISCOVERY="$TMP_ROOT/results/actual-discovery.json"
ACTUAL_CONFIG="$TMP_ROOT/results/actual-effective-config.json"
CONFIG_RESULT="$TMP_ROOT/results/config-result.json"
ORACLE_RAW="$TMP_ROOT/results/oracle-diagnostics-raw.json"
ACTUAL_RAW="$TMP_ROOT/results/actual-diagnostics-raw.json"
ORACLE_DIAGNOSTICS="$TMP_ROOT/results/oracle-diagnostics.json"
ACTUAL_DIAGNOSTICS="$TMP_ROOT/results/actual-diagnostics.json"

"$PYTHON" "$HELPER" discover "$TMP_ROOT/oracle" "$EXPECTED_DISCOVERY"
(
    cd "$TMP_ROOT/actual"
    "$KTLINT_RS" \
        --ruleset ktlint \
        --include '**/src/**/*.kt' \
        --exclude '**/generated/**' \
        --exclude '**/expected/**' \
        --print-files \
        . >"$ACTUAL_DISCOVERY"
    "$KTLINT_RS" \
        --ruleset ktlint \
        --include '**/src/**/*.kt' \
        --exclude '**/generated/**' \
        --exclude '**/expected/**' \
        --print-effective-config \
        . >"$ACTUAL_CONFIG"
)

MISMATCHES=0
if ! "$PYTHON" "$HELPER" compare-json \
    "$EXPECTED_DISCOVERY" "$ACTUAL_DISCOVERY" "$TMP_ROOT/results/discovery.diff"; then
    MISMATCHES=$((MISMATCHES + 1))
fi
if ! "$PYTHON" "$HELPER" check-config \
    "$ACTUAL_CONFIG" "$TMP_ROOT/actual/effective-config.json" "$CONFIG_RESULT"; then
    MISMATCHES=$((MISMATCHES + 1))
fi

if [[ "$OFFLINE" == true ]]; then
    cp "$TMP_ROOT/oracle/expected/diagnostics.json" "$ORACLE_RAW"
    cp "$TMP_ROOT/oracle/expected/lint-exit-code.txt" "$TMP_ROOT/results/oracle-exit-code.txt"
else
    "$GRADLE" -p "$TMP_ROOT/oracle" --no-daemon --no-configuration-cache \
        verifyOracleContract oracleLint oracleRuleInventory oracleMetadata >/dev/null
    cp "$TMP_ROOT/oracle/build/oracle/diagnostics.json" "$ORACLE_RAW"
    cp "$TMP_ROOT/oracle/build/oracle/lint-exit-code.txt" "$TMP_ROOT/results/oracle-exit-code.txt"
fi

set +e
(
    cd "$TMP_ROOT/actual"
    "$KTLINT_RS" \
        --ruleset ktlint \
        --reporter json \
        --reporter-output "$ACTUAL_RAW" \
        --include '**/src/**/*.kt' \
        --exclude '**/generated/**' \
        --exclude '**/expected/**' \
        .
)
ACTUAL_EXIT=$?
set -e
if [[ "$INJECT_MISMATCH" == "diagnostics" ]]; then
    "$PYTHON" "$HELPER" inject-diagnostic "$ACTUAL_RAW"
fi
printf '%s\n' "$ACTUAL_EXIT" >"$TMP_ROOT/results/actual-exit-code.txt"

"$PYTHON" "$HELPER" normalize-ktlint "$ORACLE_RAW" "$ORACLE_DIAGNOSTICS"
"$PYTHON" "$HELPER" normalize-rs "$ACTUAL_RAW" "$ACTUAL_DIAGNOSTICS"
if ! "$PYTHON" "$HELPER" compare-json \
    "$ORACLE_DIAGNOSTICS" "$ACTUAL_DIAGNOSTICS" "$TMP_ROOT/results/diagnostics.diff"; then
    MISMATCHES=$((MISMATCHES + 1))
fi
if ! cmp -s "$TMP_ROOT/results/oracle-exit-code.txt" "$TMP_ROOT/results/actual-exit-code.txt"; then
    MISMATCHES=$((MISMATCHES + 1))
fi

if [[ "$OFFLINE" == false ]]; then
    "$GRADLE" -p "$TMP_ROOT/oracle" --no-daemon --no-configuration-cache oracleFormat >/dev/null
    cp -R "$TMP_ROOT/oracle/src" "$TMP_ROOT/results/oracle-first-src"
    "$GRADLE" -p "$TMP_ROOT/oracle" --no-daemon --no-configuration-cache oracleFormat >/dev/null
    if ! diff -ru "$TMP_ROOT/results/oracle-first-src" "$TMP_ROOT/oracle/src" \
        >"$TMP_ROOT/results/oracle-idempotence.diff"; then
        MISMATCHES=$((MISMATCHES + 1))
    fi
elif [[ -d "$ORACLE_SOURCE/expected/formatted/src" ]]; then
    cp -R "$ORACLE_SOURCE/expected/formatted/src/." "$TMP_ROOT/oracle/src/"
fi
(
    cd "$TMP_ROOT/actual"
    "$KTLINT_RS" \
        --ruleset ktlint \
        --format \
        --include '**/src/**/*.kt' \
        --exclude '**/generated/**' \
        --exclude '**/expected/**' \
        .
)
cp -R "$TMP_ROOT/actual/src" "$TMP_ROOT/results/actual-first-src"
(
    cd "$TMP_ROOT/actual"
    "$KTLINT_RS" \
        --ruleset ktlint \
        --format \
        --include '**/src/**/*.kt' \
        --exclude '**/generated/**' \
        --exclude '**/expected/**' \
        .
)
if [[ "$INJECT_MISMATCH" == "idempotence" ]]; then
    printf '\n// injected idempotence mismatch\n' \
        >>"$TMP_ROOT/results/actual-first-src/main/kotlin/oracle/SpreadOperator.kt"
fi
if ! diff -ru "$TMP_ROOT/results/actual-first-src" "$TMP_ROOT/actual/src" \
    >"$TMP_ROOT/results/actual-idempotence.diff"; then
    MISMATCHES=$((MISMATCHES + 1))
fi
if [[ "$INJECT_MISMATCH" == "format" ]]; then
    printf '\n// Injected formatter mismatch\n' >>"$TMP_ROOT/actual/src/main/kotlin/oracle/SpreadOperator.kt"
fi

set +e
diff -ru "$TMP_ROOT/oracle/src" "$TMP_ROOT/actual/src" >"$TMP_ROOT/results/format.diff"
FORMAT_EXIT=$?
set -e
if [[ $FORMAT_EXIT -gt 1 ]]; then
    echo "diff failed with exit code $FORMAT_EXIT" >&2
    exit 2
fi
if [[ $FORMAT_EXIT -eq 1 ]]; then
    MISMATCHES=$((MISMATCHES + 1))
fi

if [[ $MISMATCHES -gt 0 ]]; then
    "$PYTHON" "$HELPER" minimize-artifacts \
        "$TMP_ROOT/input" "$TMP_ROOT/oracle" "$TMP_ROOT/actual" \
        "$ORACLE_DIAGNOSTICS" "$ACTUAL_DIAGNOSTICS" "$ARTIFACTS_DIR"
    cp -R "$TMP_ROOT/results/"* "$ARTIFACTS_DIR/"
    cp "$TMP_ROOT/oracle/oracle-manifest.json" "$ARTIFACTS_DIR/"
    cp "$TMP_ROOT/oracle/effective-config.json" "$ARTIFACTS_DIR/"
fi

if [[ "$EXPECT_MISMATCH" == true ]]; then
    if [[ $MISMATCHES -gt 0 ]]; then
        echo "Expected mismatch detected; minimized artifacts: $ARTIFACTS_DIR"
        exit 0
    fi
    echo "Expected a mismatch, but every parity surface matched." >&2
    exit 1
fi
if [[ $MISMATCHES -gt 0 ]]; then
    echo "$MISMATCHES parity surface(s) differ; artifacts: $ARTIFACTS_DIR" >&2
    exit 1
fi

echo "Discovery, config, diagnostics, exit code, formatting, and idempotence all match the Spotless oracle."
