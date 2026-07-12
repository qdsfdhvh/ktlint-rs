#!/usr/bin/env bash
# Download ktlint JVM fat jar for benchmarking.
# Usage: ./scripts/get-ktlint.sh [version]
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"

# Use specified version or fetch latest from GitHub
if [[ -n "${1:-}" ]]; then
    VERSION="$1"
else
    echo "Fetching latest ktlint version..."
    VERSION=$(curl -sS https://api.github.com/repos/ktlint/ktlint/releases/latest | grep '"tag_name"' | sed 's/.*"tag_name": "\(.*\)".*/\1/' | sed 's/^v//')
    if [[ -z "$VERSION" ]]; then
        VERSION="1.8.0"
        echo "Could not fetch latest, falling back to $VERSION"
    fi
fi

JAR="$REPO_ROOT/.ktlint/ktlint-$VERSION.jar"

if [[ -f "$JAR" ]]; then
    echo "ktlint $VERSION already at $JAR"
else
    mkdir -p "$(dirname "$JAR")"
    URL="https://github.com/ktlint/ktlint/releases/download/$VERSION/ktlint"
    echo "Downloading ktlint $VERSION from $URL ..."
    curl -sSLo "$JAR" "$URL"
    chmod +x "$JAR"
    echo "Downloaded to $JAR"
fi

# Verify and print version
java -jar "$JAR" --version
