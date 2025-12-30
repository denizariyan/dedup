#!/bin/bash
# Build and run the benchmark in Docker with all tools installed

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(dirname "$SCRIPT_DIR")"

cd "$REPO_ROOT"

echo "Building Docker image..."
docker build -t dedup-benchmark -f benchmark/Dockerfile .

echo ""
echo "Running benchmark..."
# Privileged required to drop caches before each run
docker run --rm --privileged -v "$REPO_ROOT/benchmark/results:/benchmark/results" dedup-benchmark "$@"
