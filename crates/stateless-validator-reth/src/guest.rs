//! Reth stateless validator guest program.

use alloc::vec::Vec;

use ere_platform_core::Platform;
use stateless::stateless_validation_with_trie;
use stateless_validator_common::{
    HashTreeRoot, SszEncode as _,
    guest::{
        StatelessInput, StatelessValidationResult,
        input::{
            ChainConfig, ExecutionWitness, ProtocolFork, PublicKeys,
            new_payload_request::NewPayloadRequest,
        },
    },
};
use tries::zeth::SparseState;

use crate::guest::{convert::into_validation_input, crypto::sha256_hasher, error::Error};

mod convert;
mod crypto;
mod error;

/// Runs the stateless guest on the [`Platform`].
pub fn entrypoint<P: Platform>() {
    #[cfg(feature = "zkvm-interface")]
    crypto::install_crypto();

    let input_bytes = P::read_input();
    P::write_output(&run_stateless_guest(&input_bytes));
}

/// Runs the stateless guest with serialized input and returns serialized
/// output, mirroring `run_stateless_guest` in the spec.
pub fn run_stateless_guest(input_bytes: &[u8]) -> Vec<u8> {
    let Ok((fork, input)) = StatelessInput::from_schema_prefixed_ssz(input_bytes) else {
        return StatelessValidationResult::default().to_ssz();
    };

    let StatelessInput { new_payload_request, witness, chain_config, public_keys } = input;
    let new_payload_request_root = new_payload_request.hash_tree_root(&sha256_hasher());
    let successful_validation = verify_stateless_new_payload(
        fork,
        new_payload_request,
        witness,
        &chain_config,
        public_keys,
    )
    .is_ok();

    StatelessValidationResult { new_payload_request_root, successful_validation, chain_config }
        .to_ssz()
}

/// Statelessly validates the execution payload, mirroring
/// `verify_stateless_new_payload` in the spec.
fn verify_stateless_new_payload(
    fork: ProtocolFork,
    new_payload_request: NewPayloadRequest,
    witness: ExecutionWitness,
    chain_config: &ChainConfig,
    public_keys: PublicKeys,
) -> Result<(), Error> {
    chain_config.validate(&new_payload_request)?;
    let input =
        into_validation_input(fork, new_payload_request, witness, chain_config, public_keys)?;
    stateless_validation_with_trie::<SparseState, _, _>(
        input.block,
        input.public_keys,
        input.witness,
        input.chain_spec,
        input.evm_config,
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use stateless_validator_common::SszDecode as _;

    #[test]
    fn malformed_input_returns_default_result() {
        let output = run_stateless_guest(&[]);
        let result = StatelessValidationResult::from_ssz_bytes(&output).unwrap();
        assert_eq!(result, StatelessValidationResult::default());
    }
}
