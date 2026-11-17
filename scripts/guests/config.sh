#!/usr/bin/env bash

guest_config() {
    case "${1:?zkVM name is required}" in
        openvm)
            ZKVM_VERSION="v2.1.0-preview"
            COMPILER_IMAGE="ghcr.io/eth-act/ere/ere-compiler-openvm:0.20.1@sha256:6a3a17d17a943c8145da1737d91d3c065d8f9f3ad68ef89cd46f4a26dbf199de"
            SERVER_IMAGE="ghcr.io/eth-act/ere/ere-server-openvm:0.20.1@sha256:39825cf09d567cccaa02177d8d9afc4e843dc00af679906dc02b2187195ab44b"
            ;;
        pico)
            ZKVM_VERSION="v3.1.4"
            COMPILER_IMAGE="ghcr.io/eth-act/ere/ere-compiler-pico:0.20.1@sha256:c623f3be7fecbc85fa304450e42c0c2b36677acbd47b8929cbed7c36e558f8c8"
            SERVER_IMAGE="ghcr.io/eth-act/ere/ere-server-pico:0.20.1@sha256:7ba4e4b18422e5487ee188e3464ccf2670238e31373b16b7e1d52677428eb66e"
            ;;
        sp1)
            ZKVM_VERSION="v6.9.0"
            COMPILER_IMAGE="ghcr.io/eth-act/ere/ere-compiler-sp1:0.20.1@sha256:131f3e5b920feaa4dcff4631d03ee689c6f29ce9484594c7674b7ebd602fa763"
            SERVER_IMAGE="ghcr.io/eth-act/ere/ere-server-sp1:0.20.1@sha256:955279c0632fd2a33f3d6d1a33aac1ace70587fdde7180956836193360ed015e"
            ;;
        zisk)
            ZKVM_VERSION="v1.4.0-alpha"
            COMPILER_IMAGE="ghcr.io/eth-act/ere/ere-compiler-zisk:0.20.1@sha256:a446240e353cd41fff8fed6f9a2f92ab5f69b41e07e69f020bd7f1dcc8b22b12"
            SERVER_IMAGE="ghcr.io/eth-act/ere/ere-server-zisk:0.20.1@sha256:61b04979791314d6918743d96a3ec5f60dc825f9395447ecacd4430da320e009"
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
