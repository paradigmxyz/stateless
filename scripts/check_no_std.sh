#!/usr/bin/env bash
set -eo pipefail

target=support/targets/riscv64im-a-target.json

cmd=(cargo +nightly build --no-default-features -Zjson-target-spec -Zbuild-std=core,alloc --target "$target" -p stateless)

echo "Running: ${cmd[*]}"
"${cmd[@]}"
