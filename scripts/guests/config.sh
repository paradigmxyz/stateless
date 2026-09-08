#!/usr/bin/env bash

guest_config() {
    case "${1:?zkVM name is required}" in
        openvm)
            ZKVM_VERSION="v2.1.0-preview"
            COMPILER_IMAGE="ghcr.io/eth-act/ere/ere-compiler-openvm@sha256:1d673fa4063aed15e039a0fe8cf5915d96e8fb7bd9bd7d2587d1ccf2c21e7eb1"
            SERVER_IMAGE="ghcr.io/eth-act/ere/ere-server-openvm:0.17.0@sha256:31f59d76b60223fe2f59665ebc5e8a115d6e10f52f74e5bd291f630628e57f74"
            ;;
        sp1)
            ZKVM_VERSION="v6.4.0"
            COMPILER_IMAGE="ghcr.io/eth-act/ere/ere-compiler-sp1@sha256:9c4f7fd724e1537415fa03622f006e0ac2544676100fe90e7b2363a88c67170b"
            SERVER_IMAGE="ghcr.io/eth-act/ere/ere-server-sp1:0.17.0@sha256:cdf7597dda9d2b647e1b0a88374dd801897c14812828465f9258af1eed11b903"
            ;;
        zisk)
            ZKVM_VERSION="v1.1.0-alpha"
            COMPILER_IMAGE="ghcr.io/eth-act/ere/ere-compiler-zisk@sha256:0ba9ef646d4359094e6043573530314b6717fab8c3b8bd96a7aa6329d39d172a"
            SERVER_IMAGE="ghcr.io/eth-act/ere/ere-server-zisk:0.17.0@sha256:9dc91175d0790ef42d20e03630e9beb093f53abda5d4a39cdac3de4cca588012"
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
