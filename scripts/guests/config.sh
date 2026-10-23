#!/usr/bin/env bash

guest_config() {
    case "${1:?zkVM name is required}" in
        openvm)
            ZKVM_VERSION="v2.1.0-preview"
            COMPILER_IMAGE="ghcr.io/eth-act/ere/ere-compiler-openvm:0.19.5@sha256:6b13350c2795b0f8368c74decdce0508e2552ae598771a6c86d6e449f974e165"
            SERVER_IMAGE="ghcr.io/eth-act/ere/ere-server-openvm:0.19.5@sha256:aa1cf825cb23c0ea9e850ac9b1f08311ae8195454c0289e1cc520fa127e8cea4"
            ;;
        sp1)
            ZKVM_VERSION="v6.9.0"
            COMPILER_IMAGE="ghcr.io/eth-act/ere/ere-compiler-sp1:0.19.5@sha256:a1d0b55bc2b37596bdfd129ea2545fd82dbbcd9fa3b84a68c9bc2af3859590fe"
            SERVER_IMAGE="ghcr.io/eth-act/ere/ere-server-sp1:0.19.5@sha256:82e0b809148a08f82f86ff2ddee9d352cf399a17f145754b417ecf2634578cc0"
            ;;
        zisk)
            ZKVM_VERSION="v1.3.2-alpha"
            COMPILER_IMAGE="ghcr.io/eth-act/ere/ere-compiler-zisk:0.19.5@sha256:3d93a876a9377ebf1b97b3f28effde515f6561faac686924a3ded909660db9b2"
            SERVER_IMAGE="ghcr.io/eth-act/ere/ere-server-zisk:0.19.5@sha256:569468bab192afc020c652011e68a6b06028917977d6a7567def27606d8ea96c"
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
