#!/usr/bin/env bash
set -euo pipefail

ETHEREUM_TESTS_REF="81862e4848585a438d64f911a19b3825f0f4cd95"
EEST_TESTS_TAG="v4.5.0"
ZKEVM_TESTS_TAG="zkevm@v0.3.2"

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
EF_TESTS_DIR="$SCRIPT_DIR/../testing/ef-tests"

# Clone ethereum/tests if not already present
if [ ! -d "$EF_TESTS_DIR/ethereum-tests" ]; then
    echo "Cloning ethereum/tests at $ETHEREUM_TESTS_REF..."
    git clone --depth 1 https://github.com/ethereum/tests "$EF_TESTS_DIR/ethereum-tests"
    git -C "$EF_TESTS_DIR/ethereum-tests" fetch --depth 1 origin "$ETHEREUM_TESTS_REF"
    git -C "$EF_TESTS_DIR/ethereum-tests" checkout "$ETHEREUM_TESTS_REF"
else
    echo "ethereum-tests already exists, skipping clone."
fi

# Download EEST fixtures if not already present
if [ ! -d "$EF_TESTS_DIR/execution-spec-tests" ] || [ -z "$(ls -A "$EF_TESTS_DIR/execution-spec-tests" 2>/dev/null)" ]; then
    echo "Downloading EEST fixtures ($EEST_TESTS_TAG)..."
    mkdir -p "$EF_TESTS_DIR/execution-spec-tests"
    URL="https://github.com/ethereum/execution-spec-tests/releases/download/${EEST_TESTS_TAG}/fixtures_stable.tar.gz"
    curl -L "$URL" | tar -xz --strip-components=1 -C "$EF_TESTS_DIR/execution-spec-tests"
else
    echo "execution-spec-tests already exists, skipping download."
fi

# Download zkEVM fixtures if not already present
if [ ! -d "$EF_TESTS_DIR/zkevm-tests" ] || [ -z "$(ls -A "$EF_TESTS_DIR/zkevm-tests" 2>/dev/null)" ]; then
    echo "Downloading zkEVM fixtures ($ZKEVM_TESTS_TAG)..."
    mkdir -p "$EF_TESTS_DIR/zkevm-tests"
    URL="https://github.com/ethereum/execution-spec-tests/releases/download/${ZKEVM_TESTS_TAG}/fixtures_zkevm.tar.gz"
    curl -L "$URL" | tar -xz --strip-components=1 -C "$EF_TESTS_DIR/zkevm-tests"
else
    echo "zkevm-tests already exists, skipping download."
fi

echo "EF test fixtures are ready."
