#!/usr/bin/env bash

guest_config() {
    case "${1:?zkVM name is required}" in
        openvm)
            ZKVM_VERSION="v2.1.0-preview"
            COMPILER_IMAGE="ghcr.io/eth-act/ere/ere-compiler-openvm:0.18.1@sha256:711448e51acbe0993bec3f385afe52e83d5ad598dbceb0a50bf04bb4906b4101"
            SERVER_IMAGE="ghcr.io/eth-act/ere/ere-server-openvm:0.18.1@sha256:742c94a39d413c6068c15189031739bd6b6f98cfd899255b7d283fbdbc8591b4"
            ;;
        sp1)
            ZKVM_VERSION="v6.6.0"
            COMPILER_IMAGE="ghcr.io/eth-act/ere/ere-compiler-sp1:0.18.1@sha256:964d36534d773c5019855b583f6c36800a4803580b551304a7ee271cfb786dec"
            SERVER_IMAGE="ghcr.io/eth-act/ere/ere-server-sp1:0.18.1@sha256:e486bc228be6590b54601655d2b705a2ca475021ee378fcfa3a92bd575af5925"
            ;;
        zisk)
            ZKVM_VERSION="v1.2.0-alpha"
            COMPILER_IMAGE="ghcr.io/eth-act/ere/ere-compiler-zisk:0.18.1@sha256:86200b0a3fa8669f26215ae276350d9099e847454ec2779874e9146796e08a57"
            SERVER_IMAGE="ghcr.io/eth-act/ere/ere-server-zisk:0.18.1@sha256:c4770368115dbfd557f4ccec2a0a1f9cb8cd9428ad8b0896ede553e570628415"
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
