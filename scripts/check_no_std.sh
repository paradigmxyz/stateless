#!/usr/bin/env bash
set -eo pipefail

target=riscv64im-unknown-none-elf

cmd=(cargo +stable build --no-default-features --target "$target" -p stateless)

echo "Running: ${cmd[*]}"
"${cmd[@]}"
