#!/usr/bin/env bash
set -euo pipefail

# ==============================================================================
# Run script for official Ethereum Execution Specification Tests (EEST)
# ==============================================================================

# Default trie implementation
TRIE_IMPL="default"

# Allow overriding the trie implementation via the first argument
if [ $# -gt 0 ]; then
    TRIE_IMPL="$1"
fi

# Validate the provided trie implementation
if [[ "$TRIE_IMPL" != "default" && "$TRIE_IMPL" != "zeth" ]]; then
    echo "Error: Invalid trie implementation '$TRIE_IMPL'."
    echo "Expected one of: 'default', 'zeth'"
    exit 1
fi

# Increase open file limit to prevent "Too many open files" (os error 24).
# This is critical for EF tests as they create many temporary database providers.
ulimit -n 10240

echo "-------------------------------------------------------------------------------"
echo "Running EF tests with EF_TEST_TRIE=$TRIE_IMPL"
echo "Resource Limit: ulimit -n 10240"
echo "-------------------------------------------------------------------------------"

# Prefer cargo-nextest for better test isolation and reporting, but fallback to cargo test.
if command -v cargo-nextest &> /dev/null; then
    echo "Using cargo-nextest..."
    EF_TEST_TRIE="$TRIE_IMPL" cargo nextest run \
        --no-fail-fast \
        -p ef-tests \
        --release \
        --features "asm-keccak ef-tests"
else
    echo "cargo-nextest not found, falling back to cargo test with --test-threads=1..."
    EF_TEST_TRIE="$TRIE_IMPL" cargo test \
        -p ef-tests \
        --release \
        --features "asm-keccak ef-tests" \
        -- --test-threads=1
fi
