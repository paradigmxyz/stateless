//! Reth stateless validator guest program.

use alloc::{format, vec::Vec};

pub use ere_platform_core::Platform;
use stateless::{stateless_validation_with_trie, validation::StatelessValidationError};
use stateless_validator_common::{
    HashTreeRoot, SszEncode as _,
    guest::{StatelessInput, StatelessValidationResult, input::ProtocolFork},
};
use tries::zeth::SparseState;

use crate::guest::{
    convert::{RethInput, to_reth_input},
    crypto::sha256_hasher,
};

mod convert;
pub mod crypto;
mod error;

pub use crate::guest::error::Error;

/// Runs the stateless guest on the [`Platform`].
pub fn entrypoint<P: Platform>() {
    let input_bytes = P::cycle_scope("read_input", || P::read_input());
    let output_bytes = run_stateless_guest::<P>(&input_bytes);
    P::cycle_scope("write_output", || P::write_output(&output_bytes));
}

/// Runs the stateless guest with serialized input and returns serialized
/// output, mirroring `run_stateless_guest` in the spec.
pub fn run_stateless_guest<P: Platform>(input_bytes: &[u8]) -> Vec<u8> {
    let Ok((fork, input)) = P::cycle_scope("deserialize_input", || {
        StatelessInput::from_schema_prefixed_ssz(input_bytes)
    }) else {
        return StatelessValidationResult::default().to_ssz();
    };

    let new_payload_request_root = P::cycle_scope("new_payload_request_root", || {
        input.new_payload_request.hash_tree_root(&sha256_hasher())
    });
    let chain_config = input.chain_config.clone();

    let successful_validation = verify_stateless_new_payload::<P>(fork, input).is_ok();

    let output = StatelessValidationResult::new(
        new_payload_request_root,
        successful_validation,
        chain_config,
    );

    P::cycle_scope("serialize_output", || output.to_ssz())
}

/// Statelessly validates the execution payload, mirroring
/// `verify_stateless_new_payload` in the spec.
fn verify_stateless_new_payload<P: Platform>(
    fork: ProtocolFork,
    input: StatelessInput,
) -> Result<(), Error> {
    P::cycle_scope("validate_chain_config", || {
        input.chain_config.validate(&input.new_payload_request)
    })?;

    let reth_input = P::cycle_scope("to_reth_input", || {
        to_reth_input(fork, input).map_err(|err| {
            P::print(&format!("Input conversion failed: {err}\n"));
            err
        })
    })?;

    P::cycle_scope("run_validation", || {
        run_validation(reth_input).map_err(|err| {
            P::print(&format!("Validation failed: {err}\n"));
            err
        })
    })?;

    Ok(())
}

/// Validates the reconstructed payload, reporting a rejected payload as an
/// error.
fn run_validation(input: RethInput) -> Result<(), StatelessValidationError> {
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

    #[derive(Debug)]
    struct HostPlatform;

    impl Platform for HostPlatform {
        fn read_input() -> impl core::ops::Deref<Target = [u8]> {
            &[] as &[u8]
        }

        fn write_output(_: &[u8]) {}

        fn print(_: &str) {}
    }

    #[test]
    fn malformed_input_returns_default_result() {
        let output = run_stateless_guest::<HostPlatform>(&[]);
        let result = StatelessValidationResult::from_ssz_bytes(&output).unwrap();
        assert_eq!(result, StatelessValidationResult::default());
    }
}
