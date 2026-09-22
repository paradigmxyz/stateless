#!/usr/bin/env bash
set -euo pipefail
printf 'Docker invocation reached: %s\n' "$*" >> "$GATE_DOCKER_MARKER"
echo 'TEST SENTINEL: Docker was reached; no container or dependency code executed.'
exit 90
