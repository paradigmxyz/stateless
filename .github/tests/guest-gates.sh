#!/usr/bin/env bash
set -euo pipefail

mkdir -p evidence
export CARGO_TERM_COLOR=never
REPO_ROOT="$PWD"
source scripts/guests/config.sh
echo 'Testing production build implementation from 9a573d13f5533bb1012af441c5136167ee32d05f'

if [[ "$1" == lockfile ]]; then
    guest_config "$2"
    guest_dir="bin/stateless-validator-reth/$ZKVM"
    lockfile="$REPO_ROOT/$guest_dir/Cargo.lock"
    cp "$lockfile" "evidence/$ZKVM.Cargo.lock.before"
    dependency_cache="$(mktemp -d)"
    # Fetch the unmodified real guest through Aegis before creating the fixture.
    CARGO_HOME="$dependency_cache" cargo fetch --locked --manifest-path "$guest_dir/Cargo.toml" 2>&1 | tee evidence/prefetch.log
    cmp "$lockfile" "evidence/$ZKVM.Cargo.lock.before"
    node .github/tests/guest-fixture.cjs stale-lock "$ZKVM"
    cmp "$lockfile" "evidence/$ZKVM.Cargo.lock.before"
    git diff -- "$guest_dir/Cargo.toml" | tee evidence/fixture.diff
    docker pull "$COMPILER_IMAGE"
    options=()
    if [[ "$ZKVM" == zisk ]]; then
        options+=(-e 'ERE_RUSTFLAGS=-C target-feature=+unaligned-scalar-mem')
    fi
    mkdir -p evidence/output
    # Match build.sh compiler, offline configuration, cache and read-only lock.
    # Omit host preflight AFTER mutation to test the compiler's own rejection.
    set +e
    timeout 600 docker run --rm --network none \
        -e RUST_LOG=info "${options[@]}" \
        --mount "type=bind,src=$REPO_ROOT/scripts/guests/cargo-offline.toml,dst=/usr/local/cargo/config.toml,readonly" \
        --mount "type=bind,src=$dependency_cache/registry,dst=/usr/local/cargo/registry,readonly" \
        --mount "type=bind,src=$dependency_cache/git,dst=/usr/local/cargo/git,readonly" \
        --mount "type=bind,src=$REPO_ROOT,dst=/stateless" \
        --mount "type=bind,src=$lockfile,dst=/stateless/$guest_dir/Cargo.lock,readonly" \
        --mount "type=bind,src=$REPO_ROOT/evidence/output,dst=/output" \
        "$COMPILER_IMAGE" --compiler-kind rust-customized \
        --guest-dir "/stateless/$guest_dir" --output-dir /output \
        --elf-name "$ARTIFACT_NAME.elf" -- --ignore-rust-version \
        2>&1 | tee evidence/compiler.log
    status=${PIPESTATUS[0]}
    set -e
    echo "Compiler exit status: $status"
    test "$status" -ne 0
    test "$status" -ne 124
    # Require a lockfile-specific error, not an unrelated build/cache failure.
    grep -Ei 'failed to write.*Cargo.lock|failed to (write|open).*lock file|lock file.*needs to be updated' evidence/compiler.log
    grep -Ei 'Read-only file system|read.only|--locked' evidence/compiler.log
    cmp "$lockfile" "evidence/$ZKVM.Cargo.lock.before"
    test ! -e "evidence/output/$ARTIFACT_NAME.elf"
    echo "PASS: $ZKVM rejected the required lockfile change; lockfile unchanged; no ELF." | tee evidence/result.txt
    printf '### %s lockfile negative test\n\n%s\n' "$ZKVM" "$(<evidence/result.txt)" >> "$GITHUB_STEP_SUMMARY"
elif [[ "$1" == cooldown ]]; then
    sentinel_dir="$(mktemp -d)"
    install -m 755 .github/tests/docker-sentinel.sh "$sentinel_dir/docker"
    export GATE_DOCKER_MARKER="$REPO_ROOT/evidence/docker-control.marker"
    node .github/tests/guest-fixture.cjs cooldown sp1 itoa 1.0.18
    cargo generate-lockfile --manifest-path bin/stateless-validator-reth/sp1/Cargo.toml
    cp bin/stateless-validator-reth/sp1/Cargo.lock evidence/control.Cargo.lock
    set +e
    PATH="$sentinel_dir:$PATH" bash scripts/guests/build.sh sp1 evidence/control-output 2>&1 | tee evidence/control.log
    status=${PIPESTATUS[0]}
    set -e
    test "$status" -eq 90
    test -s "$GATE_DOCKER_MARKER"
    echo 'PASS: cooled itoa 1.0.18 fetched successfully and reached Docker sentinel.'

    export GATE_DOCKER_MARKER="$REPO_ROOT/evidence/docker-negative.marker"
    # cc 1.4.7 upstream release: 2026-09-18; test only, never compile it.
    node .github/tests/guest-fixture.cjs cooldown sp1 cc 1.4.7
    cargo generate-lockfile --manifest-path bin/stateless-validator-reth/sp1/Cargo.toml
    cp bin/stateless-validator-reth/sp1/Cargo.lock evidence/negative.Cargo.lock
    set +e
    PATH="$sentinel_dir:$PATH" bash scripts/guests/build.sh sp1 evidence/negative-output 2>&1 | tee evidence/cooldown.log
    status=${PIPESTATUS[0]}
    set -e
    echo "Uncooled fetch/build script exit status: $status"
    test "$status" -ne 0
    test "$status" -ne 90
    test ! -e "$GATE_DOCKER_MARKER"
    grep -Ei 'cooldown|too.new|minimum.age' evidence/cooldown.log
    cmp bin/stateless-validator-reth/sp1/Cargo.lock evidence/negative.Cargo.lock
    echo 'PASS: real cooldown rejection made host fetch fail; build.sh never invoked Docker.' | tee evidence/result.txt
    printf '### Cooldown gate negative test\n\n%s\n' "$(<evidence/result.txt)" >> "$GITHUB_STEP_SUMMARY"
else
    exit 2
fi
