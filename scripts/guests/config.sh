#!/usr/bin/env bash

guest_config() {
    case "${1:?zkVM name is required}" in
        openvm)
            ZKVM_VERSION="v2.1.0-preview"
            COMPILER_IMAGE="ghcr.io/eth-act/ere/ere-compiler-openvm@sha256:1d673fa4063aed15e039a0fe8cf5915d96e8fb7bd9bd7d2587d1ccf2c21e7eb1"
            SERVER_IMAGE="ghcr.io/eth-act/ere/ere-server-openvm@sha256:d56f5f59f1560bc2b58e84165accb8dc6586596ca1b9ff53b99862560f7b563c"
            ;;
        sp1)
            ZKVM_VERSION="v6.4.0"
            COMPILER_IMAGE="ghcr.io/eth-act/ere/ere-compiler-sp1@sha256:9c4f7fd724e1537415fa03622f006e0ac2544676100fe90e7b2363a88c67170b"
            SERVER_IMAGE="ghcr.io/eth-act/ere/ere-server-sp1@sha256:561ba9d13e0f4f198c57ad8c76b8320f691635275f6567bcb27b36d4e4159af4"
            ;;
        zisk)
            ZKVM_VERSION="v1.1.0-alpha"
            COMPILER_IMAGE="ghcr.io/eth-act/ere/ere-compiler-zisk@sha256:0ba9ef646d4359094e6043573530314b6717fab8c3b8bd96a7aa6329d39d172a"
            SERVER_IMAGE="ghcr.io/eth-act/ere/ere-server-zisk@sha256:4d00a96890a26dc54b523e537e9c1b25a26cf18d166bd3614fc97bbd09317091"
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
