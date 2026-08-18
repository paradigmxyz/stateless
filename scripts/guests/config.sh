#!/usr/bin/env bash

guest_config() {
    case "${1:?zkVM name is required}" in
        openvm)
            ZKVM_VERSION="v2.1.0-preview"
            COMPILER_IMAGE="ghcr.io/eth-act/ere/ere-compiler-openvm@sha256:6afea444a0470641d1bdc7aae43b80f022cc55af72c12d64c8f0e9419f195cf1"
            SERVER_IMAGE="ghcr.io/eth-act/ere/ere-server-openvm@sha256:57d912b9531b40839774b4d4d81a4a7d22e76ef0a8283ec58c8de4f7fb78cbfb"
            ;;
        sp1)
            ZKVM_VERSION="v6.3.1"
            COMPILER_IMAGE="ghcr.io/eth-act/ere/ere-compiler-sp1@sha256:0bf8394ba9487b1124e5ea6abc7debf3afc459168dcd51e8127c423fe6f5689a"
            SERVER_IMAGE="ghcr.io/eth-act/ere/ere-server-sp1@sha256:170f6641707015dddc5a90e7ac71a511a3ccbff16082d785e13c21fde67e0744"
            ;;
        zisk)
            ZKVM_VERSION="v1.0.0-alpha"
            COMPILER_IMAGE="ghcr.io/eth-act/ere/ere-compiler-zisk@sha256:e5fea7055c6f4f005d1c1e0fd90e20abc171c0dc00ca0b75af7272004eb761e8"
            SERVER_IMAGE="ghcr.io/eth-act/ere/ere-server-zisk@sha256:1fd910170d25290d547a16600927b9735d619354f0cab1c306ef06412e09d46e"
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
