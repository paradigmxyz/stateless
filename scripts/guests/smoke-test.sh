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
PORT=4174
CONTAINER_NAME="reth-guest-smoke-$ZKVM"
ENDPOINT="http://127.0.0.1:$PORT/"

docker pull "$SERVER_IMAGE"
docker image inspect "$SERVER_IMAGE" >/dev/null

cargo build --locked \
    --package stateless-validator-tests \
    --bin guest-smoke

docker rm --force "$CONTAINER_NAME" >/dev/null 2>&1 || true
trap 'docker rm --force "$CONTAINER_NAME" >/dev/null 2>&1 || true' EXIT

docker_options=(
    run
    --detach
    --name "$CONTAINER_NAME"
    --publish "127.0.0.1:$PORT:$PORT"
    --env RUST_LOG=info
    --mount "type=bind,src=$ELF,dst=/guest.elf,readonly"
)
case "$ZKVM" in
    sp1)
        docker_options+=(--shm-size 32G --env ERE_SP1_EXECUTOR_POOL_SIZE=1)
        ;;
    zisk)
        docker_options+=(--shm-size 32G --ulimit memlock=-1:-1)
        ;;
esac

docker "${docker_options[@]}" "$SERVER_IMAGE" \
    --port "$PORT" \
    --elf-path /guest.elf \
    cpu >/dev/null

deadline=$((SECONDS + 15 * 60))
until curl --fail --silent --show-error --connect-timeout 2 --max-time 3 \
    "${ENDPOINT}health" >/dev/null 2>&1; do
    state="$(docker inspect --format '{{.State.Status}}' "$CONTAINER_NAME")"
    if [[ "$state" != "running" ]]; then
        echo "$CONTAINER_NAME stopped while initializing" >&2
        docker logs "$CONTAINER_NAME" >&2
        exit 1
    fi
    if ((SECONDS >= deadline)); then
        echo "$CONTAINER_NAME did not become healthy within 15 minutes" >&2
        docker logs "$CONTAINER_NAME" >&2
        exit 1
    fi
    sleep 2
done

if ! "$REPO_ROOT/target/debug/guest-smoke" "$ZKVM" "$ENDPOINT" "$VK" "$FIXTURE"; then
    docker logs "$CONTAINER_NAME" >&2
    exit 1
fi
