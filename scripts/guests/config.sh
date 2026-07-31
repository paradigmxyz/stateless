#!/usr/bin/env bash

guest_config() {
    case "${1:?zkVM name is required}" in
        openvm)
            ZKVM_VERSION="v2.0.0"
            COMPILER_IMAGE="ghcr.io/eth-act/ere/ere-compiler-openvm@sha256:0e903d5569b69e756e28ce0e502782573d529d9ea31623d3e5bf0dd4d336a188"
            SERVER_IMAGE="ghcr.io/eth-act/ere/ere-server-openvm@sha256:c0f183c9fd18dcad0a7bc37281fab27b5f0b328cad6960afaa6b673cc4c2b063"
            ;;
        sp1)
            ZKVM_VERSION="v6.3.1"
            COMPILER_IMAGE="ghcr.io/eth-act/ere/ere-compiler-sp1@sha256:481da6fd7fb2f3b644e39314e423ddc621597e5ad7a108fccb29523fa704f7ba"
            SERVER_IMAGE="ghcr.io/eth-act/ere/ere-server-sp1@sha256:84443bbf6497061c81fe4288a093b89ec2b1bf79d0f31df61bf1c9726d527fc3"
            ;;
        zisk)
            ZKVM_VERSION="v1.0.0-alpha"
            COMPILER_IMAGE="ghcr.io/eth-act/ere/ere-compiler-zisk@sha256:c5e6b80c5f76fd286d43d7ed0d580137d7b9a55b26e8f4d07f8a13c170c6c411"
            SERVER_IMAGE="ghcr.io/eth-act/ere/ere-server-zisk@sha256:fffe0f3b42275502b47ecd153de426e85fed64cee75631e98369a4c89dd84a31"
            ;;
        *)
            echo "unsupported zkVM: $1" >&2
            return 2
            ;;
    esac

    ZKVM="$1"
    ARTIFACT_NAME="stateless-validator-reth-${ZKVM}-${ZKVM_VERSION}"
    export ZKVM ZKVM_VERSION COMPILER_IMAGE SERVER_IMAGE ARTIFACT_NAME
}

sha256_file() {
    if command -v sha256sum >/dev/null 2>&1; then
        sha256sum "$1" | awk '{print $1}'
    else
        shasum -a 256 "$1" | awk '{print $1}'
    fi
}
