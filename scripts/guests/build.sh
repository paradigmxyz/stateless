#!/usr/bin/env bash

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(git -C "$SCRIPT_DIR" rev-parse --show-toplevel)"
# shellcheck source=config.sh
source "$SCRIPT_DIR/config.sh"

if [[ $# -lt 1 || $# -gt 2 ]]; then
    echo "usage: $0 <openvm|sp1|zisk> [output-directory]" >&2
    exit 2
fi

guest_config "$1"
OUTPUT_DIR="${2:-$REPO_ROOT/output}"
mkdir -p "$OUTPUT_DIR"
OUTPUT_DIR="$(cd "$OUTPUT_DIR" && pwd)"

LOCKFILE="$REPO_ROOT/bin/stateless-validator-reth/$ZKVM/Cargo.lock"
CONTAINER_GUEST_DIR="/stateless/bin/stateless-validator-reth/$ZKVM"

docker run --rm \
    -e RUST_LOG=info \
    --mount "type=bind,src=$REPO_ROOT,dst=/stateless" \
    --mount "type=bind,src=$LOCKFILE,dst=$CONTAINER_GUEST_DIR/Cargo.lock,readonly" \
    --mount "type=bind,src=$OUTPUT_DIR,dst=/output" \
    "$COMPILER_IMAGE" \
    --compiler-kind rust-customized \
    --guest-dir "$CONTAINER_GUEST_DIR" \
    --output-dir /output \
    --elf-name "$ARTIFACT_NAME.elf" \
    -- \
    --ignore-rust-version

docker run --rm \
    -e RUST_LOG=info \
    --mount "type=bind,src=$OUTPUT_DIR,dst=/output" \
    "$SERVER_IMAGE" \
    --elf-path "/output/$ARTIFACT_NAME.elf" \
    keygen \
    --program-vk-path "/output/$ARTIFACT_NAME.vk"

ELF="$OUTPUT_DIR/$ARTIFACT_NAME.elf"
VK="$OUTPUT_DIR/$ARTIFACT_NAME.vk"
if [[ "$(od -An -t x1 -N4 "$ELF" | tr -d ' \n')" != "7f454c46" ]]; then
    echo "$ELF is not an ELF file" >&2
    exit 1
fi
if [[ ! -s "$VK" ]]; then
    echo "$VK is empty" >&2
    exit 1
fi

echo "Built $ARTIFACT_NAME"
