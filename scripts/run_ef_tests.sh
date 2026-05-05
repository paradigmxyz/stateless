#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

if [[ $# -ne 1 ]]; then
  echo "Usage: $0 <default|zeth>"
  exit 1
fi

TRIE_IMPL="$1"
case "$TRIE_IMPL" in
default|zeth) ;;
*)
  echo "Invalid trie implementation: $TRIE_IMPL"
  echo "Expected one of: default, zeth"
  exit 1
  ;;
esac

# Setup test fixtures
"$SCRIPT_DIR/setup_ef_tests.sh" --force

# MacOS specific setup to avoid 'too many files open' and 'stack overflow' errors
# ulimit -n 65536
# export RUST_MIN_STACK=16777216
# export RAYON_NUM_THREADS=1

# Run EF tests
EF_TEST_TRIE="$TRIE_IMPL" cargo nextest run --no-fail-fast -p ef-tests --release --features "asm-keccak ef-tests"
