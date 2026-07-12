#!/usr/bin/env bash
# Download ktlint JVM fat jar for benchmarking.
# Usage: ./scripts/get-ktlint.sh [version]
set -euo pipefail

VERSION="${1:-1.5.0}"
REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
JAR="$REPO_ROOT/.ktlint/ktlint-$VERSION.jar"

if [[ -f "$JAR" ]]; then
    echo "ktlint $VERSION already at $JAR"
else
    mkdir -p "$(dirname "$JAR")"
    URL="https://github.com/pinterest/ktlint/releases/download/$VERSION/ktlint"
    echo "Downloading ktlint $VERSION from $URL ..."
    curl -sSLo "$JAR" "$URL"
    chmod +x "$JAR"
    echo "Downloaded to $JAR"
fi

# Verify and print version
java -jar "$JAR" --version
