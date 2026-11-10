#!/usr/bin/env bash

guest_config() {
    case "${1:?zkVM name is required}" in
        openvm)
            ZKVM_VERSION="v2.1.0-preview"
            COMPILER_IMAGE="ghcr.io/eth-act/ere/ere-compiler-openvm:0.20.0@sha256:bd721f337b91cac2beffd560377145f7ee8f77dd3b235642c48cbbceef5547f5"
            SERVER_IMAGE="ghcr.io/eth-act/ere/ere-server-openvm:0.20.0@sha256:db58fed53e626ca791db4940ced55957ec04060ded912a4b01463d0092ffae4e"
            ;;
        sp1)
            ZKVM_VERSION="v6.9.0"
            COMPILER_IMAGE="ghcr.io/eth-act/ere/ere-compiler-sp1:0.20.0@sha256:3612ee81acf2a97d12e30229e872881f517affaf3174108de567a648252c1b25"
            SERVER_IMAGE="ghcr.io/eth-act/ere/ere-server-sp1:0.20.0@sha256:c5bebed6b2db4ae395d44c1b67deaef0fe24293fccdd352470e69295ff777c8e"
            ;;
        zisk)
            ZKVM_VERSION="v1.4.0-alpha"
            COMPILER_IMAGE="ghcr.io/eth-act/ere/ere-compiler-zisk:0.20.0@sha256:abb8c528cda7cb0e8c8f85607464d808012f5248d17d0c6a285c3c43e1d61e3c"
            SERVER_IMAGE="ghcr.io/eth-act/ere/ere-server-zisk:0.20.0@sha256:008d7f64430f2f3f62770ac60c79c77652d75203a60d8034a59f43a1ff240d3b"
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
