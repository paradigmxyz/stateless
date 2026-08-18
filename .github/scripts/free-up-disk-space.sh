#!/usr/bin/env bash

set -euo pipefail

df -h /
sudo rm -rf /usr/share/dotnet
sudo rm -rf /usr/local/lib/android
sudo rm -rf /opt/ghc /usr/local/.ghcup
sudo rm -rf /opt/hostedtoolcache
df -h /
