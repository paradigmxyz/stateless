#!/usr/bin/env bash
set -eo pipefail

target=targets/riscv64im-unknown-none-elf.json

# Required to prevent __atomic_* references from ending up in the final rlib.
export RUSTFLAGS="-C passes=lower-atomic"

cmd=(
  cargo +nightly build
  --no-default-features
  --target "$target"
  -Zbuild-std=core,alloc
  -Zjson-target-spec
  -p stateless
)

echo "Running: ${cmd[*]}"
"${cmd[@]}"
