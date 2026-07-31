#!/usr/bin/env bash
set -euo pipefail

target=targets/riscv64im-unknown-none-elf.json

# Required to prevent __atomic_* references from ending up in the final rlib.
export RUSTFLAGS="-C passes=lower-atomic"

build() {
  local package="$1"
  local features="${2:-}"
  local -a cmd=(
    cargo +nightly build
    --locked
    --no-default-features
    --target "$target"
    "-Zbuild-std=core,alloc"
    -Zjson-target-spec
    -p "$package"
  )

  if [[ -n "$features" ]]; then
    cmd+=(--features "$features")
  fi

  echo "Running: ${cmd[*]}"
  "${cmd[@]}"
}

build stateless
build stateless zkvm-interface
build stateless-validator-common
build stateless-validator-reth
build stateless-validator-reth zkvm-interface
