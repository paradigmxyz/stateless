#!/usr/bin/env bash

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(git -C "$SCRIPT_DIR" rev-parse --show-toplevel)"
# shellcheck source=config.sh
source "$SCRIPT_DIR/config.sh"

if [[ $# -gt 1 ]]; then
    echo "usage: $0 [artifact-directory]" >&2
    exit 2
fi

ARTIFACT_DIR="${1:-$REPO_ROOT/output}"
ARTIFACT_DIR="$(cd "$ARTIFACT_DIR" && pwd)"
FILES="[]"
GUESTS="[]"

for ZKVM_NAME in openvm sp1 zisk; do
    guest_config "$ZKVM_NAME"
    LOCKFILE="bin/stateless-validator-reth/$ZKVM_NAME/Cargo.lock"
    GUESTS="$(
        jq -c \
            --arg zkvm "$ZKVM_NAME" \
            --arg version "$ZKVM_VERSION" \
            --arg compiler_image "$COMPILER_IMAGE" \
            --arg server_image "$SERVER_IMAGE" \
            --arg lockfile "$LOCKFILE" \
            --arg lockfile_sha256 "$(sha256_file "$REPO_ROOT/$LOCKFILE")" \
            '. + [{
                zkvm: $zkvm,
                version: $version,
                compiler_image: $compiler_image,
                server_image: $server_image,
                lockfile: $lockfile,
                lockfile_sha256: $lockfile_sha256
            }]' <<< "$GUESTS"
    )"
done

while IFS= read -r FILE; do
    FILES="$(
        jq -c \
            --arg name "$(basename "$FILE")" \
            --arg sha256 "$(sha256_file "$FILE")" \
            --argjson size "$(wc -c < "$FILE" | tr -d ' ')" \
            '. + [{name: $name, size: $size, sha256: $sha256}]' <<< "$FILES"
    )"
done < <(find "$ARTIFACT_DIR" -maxdepth 1 -type f \( -name '*.elf' -o -name '*.vk' \) | sort)

if [[ "$(jq length <<< "$FILES")" -ne 6 ]]; then
    echo "expected three ELF and three VK files in $ARTIFACT_DIR" >&2
    exit 1
fi

jq -n \
    --arg repository "${GITHUB_REPOSITORY:-paradigmxyz/stateless}" \
    --arg source_commit "${GITHUB_SHA:-$(git -C "$REPO_ROOT" rev-parse HEAD)}" \
    --arg release_tag "${GITHUB_REF_NAME:-}" \
    --arg ere_version "v0.14.0" \
    --arg input_contract "execution-specs statelessInputBytes/statelessOutputBytes" \
    --argjson guests "$GUESTS" \
    --argjson files "$FILES" \
    '{
        repository: $repository,
        source_commit: $source_commit,
        release_tag: $release_tag,
        ere_version: $ere_version,
        input_contract: $input_contract,
        guests: $guests,
        files: $files
    }' > "$ARTIFACT_DIR/RELEASE-MANIFEST.json"
