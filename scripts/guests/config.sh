#!/usr/bin/env bash

guest_config() {
    case "${1:?zkVM name is required}" in
        openvm)
            ZKVM_VERSION="v2.1.0-preview"
            COMPILER_IMAGE="ghcr.io/eth-act/ere/ere-compiler-openvm:c0a4126@sha256:761ddf4f7b21ed9151e1a2cda13cf36bf02d3eecd075f3d8a921beda4ac688c3"
            SERVER_IMAGE="ghcr.io/eth-act/ere/ere-server-openvm:c0a4126@sha256:1554f023fb3c8c3fc7832abfa34312de2670a95a1351485e0cc34c0d03dbbfbe"
            ;;
        pico)
            ZKVM_VERSION="v3.1.4"
            COMPILER_IMAGE="ghcr.io/eth-act/ere/ere-compiler-pico:c0a4126@sha256:ffa86ab9a51de9d72aed4a7d3b4ce1ea4290b97f9ad5ab4cf0bb0c508831c7a2"
            SERVER_IMAGE="ghcr.io/eth-act/ere/ere-server-pico:c0a4126@sha256:48de20fd253dfb94d5644b5cffb4ef099208856d7361ec8afef74b7a61c8a68c"
            ;;
        sp1)
            ZKVM_VERSION="v6.9.0"
            COMPILER_IMAGE="ghcr.io/eth-act/ere/ere-compiler-sp1:c0a4126@sha256:704b748b8a9c1f73112cc79f71e4d08346ef0c1aecb0add87957f7f89adf3349"
            SERVER_IMAGE="ghcr.io/eth-act/ere/ere-server-sp1:c0a4126@sha256:ed3a0711161434ab45b4f84a8be62816fb4ea9197773f78d251f078d66061cfc"
            ;;
        zisk)
            ZKVM_VERSION="v1.4.0-alpha"
            COMPILER_IMAGE="ghcr.io/eth-act/ere/ere-compiler-zisk:c0a4126@sha256:895c4bad8a0c8eb00535d54c6e8469c9d28eb1feefeff85b2df13789a4d85c48"
            SERVER_IMAGE="ghcr.io/eth-act/ere/ere-server-zisk:c0a4126@sha256:f403d5e5c352c1095eb5e67f443df73b2aeceb88fb699c95b6d68123596c694d"
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
