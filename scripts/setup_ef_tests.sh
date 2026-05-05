#!/usr/bin/env bash
set -euo pipefail

# ==============================================================================
# Setup script for official Ethereum Execution Specification Tests (EEST)
# This script implements the new witness generation flow using the
# execution-specs repository and the 'uv' tool.
# ==============================================================================

# Paths
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
EF_TESTS_DIR="$SCRIPT_DIR/../testing/ef-tests"
EF_SPECS_DIR="$SCRIPT_DIR/../testing/ef-specs"
FIXTURES_DEST="$EF_TESTS_DIR/execution-spec-tests"

# Options
FILL_ALL=false

if [[ "${1:-}" == "--all" ]]; then
    FILL_ALL=true
fi

# Ensure dependencies are installed
if ! command -v uv &> /dev/null; then
    echo "Error: 'uv' is not installed. Please install it via: curl -LsSf https://astral.sh/uv/install.sh | sh"
    exit 1
fi

# 1. Clone ethereum/tests (Legacy requirement for general test structure)
if [ ! -d "$EF_TESTS_DIR/ethereum-tests" ]; then
    echo "Cloning ethereum/tests..."
    mkdir -p "$EF_TESTS_DIR"
    git clone --depth 1 https://github.com/ethereum/tests "$EF_TESTS_DIR/ethereum-tests"
else
    echo "ethereum-tests already exists, skipping clone."
fi

# 2. Setup execution-specs repository
if [ ! -d "$EF_SPECS_DIR" ]; then
    echo "Cloning execution-specs repository..."
    git clone https://github.com/ethereum/execution-specs "$EF_SPECS_DIR"
    cd "$EF_SPECS_DIR"
    git checkout projects/zkevm-bal-devnet-3
    cd "$SCRIPT_DIR/.."
else
    echo "execution-specs already exists, skipping clone."
    cd "$EF_SPECS_DIR"
    git checkout projects/zkevm-bal-devnet-3
    cd "$SCRIPT_DIR/.."
fi

# 3. Fill fixtures using the official specification tool
echo "Generating execution witnesses using uv run fill..."
cd "$EF_SPECS_DIR"

# Priority 1: Fill the most focused cases (EIP-8025 optional proofs) - Always run
echo "Filling focused Amsterdam eip8025_optional_proofs tests..."
uv run fill --clean -m "blockchain_test" --fork Amsterdam -s ./tests/amsterdam/eip8025_optional_proofs

# Priority 2: Fill the general EEST cases (>16k tests) - Optional
if [ "$FILL_ALL" = true ]; then
    echo "Filling general blockchain test cases (all forks)..."
    uv run fill --clean -m "blockchain_test"
else
    echo "Skipping general blockchain test cases (use --all to enable)."
fi

cd "$SCRIPT_DIR/.."

# 4. Synchronize filled fixtures to the ef-tests directory
echo "Syncing filled fixtures to $FIXTURES_DEST..."
mkdir -p "$FIXTURES_DEST"

# We copy the contents of the fixtures/blockchain_tests directory from the specs repo.
# The Rust test runner uses WalkDir, so preserving the directory structure is important.
cp -r "$EF_SPECS_DIR/fixtures/blockchain_tests/"* "$FIXTURES_DEST/"

echo "Official EF test fixtures are ready."
echo "You can now run tests using: EF_TEST_TRIE=default cargo test -p ef-tests --release --features \"asm-keccak ef-tests\""
echo "To generate all fork fixtures next time, run: $0 --all"
