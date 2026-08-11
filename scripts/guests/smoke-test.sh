#!/usr/bin/env bash

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(git -C "$SCRIPT_DIR" rev-parse --show-toplevel)"
# shellcheck source=config.sh
source "$SCRIPT_DIR/config.sh"

if [[ $# -lt 1 || $# -gt 2 ]]; then
    echo "usage: $0 <openvm|sp1|zisk> [artifact-directory]" >&2
    exit 2
fi

guest_config "$1"
OUTPUT_DIR="${2:-$REPO_ROOT/output}"
ELF="$OUTPUT_DIR/$ARTIFACT_NAME.elf"
VK="$OUTPUT_DIR/$ARTIFACT_NAME.vk"
FIXTURE="$REPO_ROOT/testing/stateless-validator/fixtures/guest-smoke.json"
IMAGE_REGISTRY="ghcr.io/eth-act/ere"
SERVER_TAG="$IMAGE_REGISTRY/ere-server-$ZKVM:0.15.0"

# DockerizedzkVM resolves an ERE release tag. Point that local tag at the digest-pinned image used
# for key generation so the smoke test cannot execute against different runtime bytes.
docker pull "$SERVER_IMAGE"
docker image inspect "$SERVER_IMAGE" >/dev/null
docker tag "$SERVER_IMAGE" "$SERVER_TAG"

ERE_IMAGE_REGISTRY="$IMAGE_REGISTRY" cargo run --locked \
    --package stateless-validator-tests \
    --bin guest-smoke \
    -- "$ZKVM" "$ELF" "$VK" "$FIXTURE"
