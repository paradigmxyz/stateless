//! Canonical stateless validation input types.
//!
//! The types mirror [`stateless.py`] and their SSZ schemas in [`stateless_ssz.py`]. The wire
//! format is a 2-byte big-endian schema identifier followed by the SSZ-encoded `StatelessInput`
//! container.
//!
//! [`stateless.py`]: https://github.com/ethereum/execution-specs/blob/tests-zkevm@v0.6.2/src/ethereum/forks/amsterdam/stateless.py
//! [`stateless_ssz.py`]: https://github.com/ethereum/execution-specs/blob/tests-zkevm@v0.6.2/src/ethereum/forks/amsterdam/stateless_ssz.py

#![allow(missing_docs)]

use alloc::vec::Vec;
use core::fmt::{self, Debug};

use libssz::{SszDecode, SszEncode};
use libssz_derive::{SszDecode, SszEncode};
use libssz_types::SszList;

use crate::guest::{error::Error, input::new_payload_request::NewPayloadRequest};

pub mod new_payload_request;

/// Revision byte of the SSZ stateless input schema.
///
/// The schema identifier is `fork_index << 8 | revision`, where `fork_index` is the
/// [`ProtocolFork`] discriminant of the payload's active fork.
///
/// The spec fixes the fork index to Amsterdam. This implementation extends it
/// and accepts payload shape from Bellatrix onward under the matching identifier.
pub const STATELESS_INPUT_SCHEMA_REVISION: u8 = 0x01;
/// Byte length of the big-endian schema identifier prefix.
pub const STATELESS_INPUT_SCHEMA_ID_SIZE: usize = 2;

/// SSZ list bounds from the Amsterdam stateless schema.
pub const MAX_WITNESS_NODES: usize = 1 << 22;
pub const MAX_WITNESS_CODES: usize = 1 << 18;
pub const MAX_WITNESS_HEADERS: usize = 256;
pub const MAX_BYTES_PER_WITNESS_NODE: usize = 1 << 10;
pub const MAX_BYTES_PER_CODE: usize = 1 << 16;
pub const MAX_BYTES_PER_HEADER: usize = 1 << 10;
pub const MAX_OPTIONAL_FORK_ACTIVATION_VALUES: usize = 1;
pub const MAX_PUBLIC_KEYS: usize = 1 << 15;
pub const PUBLIC_KEY_BYTES: usize = 65;

/// Transaction public keys in payload order.
pub type PublicKeys = SszList<[u8; PUBLIC_KEY_BYTES], MAX_PUBLIC_KEYS>;

/// Execution witness data for stateless validation.
#[derive(Debug, Clone, Default, PartialEq, Eq, SszEncode, SszDecode)]
pub struct ExecutionWitness {
    /// Hashed trie-node preimages needed during execution and state-root recomputation.
    pub state: SszList<SszList<u8, MAX_BYTES_PER_WITNESS_NODE>, MAX_WITNESS_NODES>,
    /// Contract-code preimages (created or accessed) needed during execution.
    pub codes: SszList<SszList<u8, MAX_BYTES_PER_CODE>, MAX_WITNESS_CODES>,
    /// RLP-encoded block headers used for pre-state and `BLOCKHASH` correctness proofs. This may
    /// trend toward empty EIP-7709.
    pub headers: SszList<SszList<u8, MAX_BYTES_PER_HEADER>, MAX_WITNESS_HEADERS>,
}

/// Execution-layer fork identifiers used by stateless schemas.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u64)]
pub enum ProtocolFork {
    Paris = 0x0E,
    Shanghai = 0x0F,
    Cancun = 0x10,
    Prague = 0x11,
    Osaka = 0x12,
    BPO1 = 0x13,
    BPO2 = 0x14,
    Amsterdam = 0x15,
}

impl ProtocolFork {
    /// Converts an SSZ enum value into a [`ProtocolFork`].
    pub fn from_u64(value: u64) -> Option<Self> {
        Some(match value {
            0x0E => Self::Paris,
            0x0F => Self::Shanghai,
            0x10 => Self::Cancun,
            0x11 => Self::Prague,
            0x12 => Self::Osaka,
            0x13 => Self::BPO1,
            0x14 => Self::BPO2,
            0x15 => Self::Amsterdam,
            _ => return None,
        })
    }

    /// Returns the SSZ enum value of this fork.
    pub fn as_u64(self) -> u64 {
        self as u64
    }
}

/// Activation point for a protocol fork.
///
/// The spec models both fields as optional values where at least one must be
/// set. The SSZ schema encodes each as a list holding zero or one element.
#[derive(Clone, Default, PartialEq, Eq, SszEncode, SszDecode)]
pub struct ForkActivation {
    pub block_number: SszList<u64, MAX_OPTIONAL_FORK_ACTIVATION_VALUES>,
    pub timestamp: SszList<u64, MAX_OPTIONAL_FORK_ACTIVATION_VALUES>,
}

impl ForkActivation {
    /// Returns the activation block number when present.
    pub fn block_number(&self) -> Option<u64> {
        self.block_number.first().copied()
    }

    /// Returns the activation timestamp when present.
    pub fn timestamp(&self) -> Option<u64> {
        self.timestamp.first().copied()
    }

    /// Returns whether this activation point is active for a payload, applying the block-number
    /// and timestamp comparisons of `_is_activation_active` in the spec. The both-unset case, on
    /// which the spec raises, is rejected earlier by [`ChainConfig::validate`] and yields `false`
    /// here.
    pub fn is_active_at(&self, block_number: u64, timestamp: u64) -> bool {
        let activation_block_number = self.block_number();
        let activation_timestamp = self.timestamp();
        if activation_block_number.is_none() && activation_timestamp.is_none() {
            return false;
        }
        if activation_block_number.is_some_and(|at| block_number < at) {
            return false;
        }
        if activation_timestamp.is_some_and(|at| timestamp < at) {
            return false;
        }
        true
    }
}

impl Debug for ForkActivation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ForkActivation")
            .field("block_number", &self.block_number.first())
            .field("timestamp", &self.timestamp.first())
            .finish()
    }
}

/// Per-fork configuration needed to interpret stateless inputs.
#[derive(Debug, Clone, Default, PartialEq, Eq, SszEncode, SszDecode)]
pub struct ForkConfig {
    pub activation: ForkActivation,
}

/// Chain configuration needed for stateless validation.
#[derive(Debug, Clone, Default, PartialEq, Eq, SszEncode, SszDecode)]
pub struct ChainConfig {
    pub chain_id: u64,
    pub active_fork: ForkConfig,
}

impl ChainConfig {
    /// Validates that the chain configuration is usable for the target payload, following
    /// `validate_chain_config` in the spec. The active fork is selected by the schema identifier
    /// during [`StatelessInput`] decoding, so only the activation point is checked here.
    pub fn validate(&self, new_payload_request: &NewPayloadRequest) -> Result<(), Error> {
        if self.active_fork.activation.block_number().is_none()
            && self.active_fork.activation.timestamp().is_none()
        {
            return Err(Error::InvalidForkActivation);
        }

        if !self
            .active_fork
            .activation
            .is_active_at(new_payload_request.block_number(), new_payload_request.timestamp())
        {
            return Err(Error::InactiveForkConfig);
        }

        Ok(())
    }
}

/// Canonical input to stateless validation.
///
/// A fork-agnostic SSZ container. The active fork is in the 2-byte schema identifier rather than
/// the SSZ body, and decoding validates the recovered payload request against that fork.
#[derive(Debug, Clone, PartialEq, Eq, SszEncode)]
pub struct StatelessInput {
    /// Consensus-layer payload request to validate statelessly. See [`NewPayloadRequest`] for
    /// structure and links to consensus-specs.
    pub new_payload_request: NewPayloadRequest,
    /// Execution witness material required to re-execute the core state transition function
    /// statelessly.
    pub witness: ExecutionWitness,
    /// Chain configuration values needed during stateless validation.
    pub chain_config: ChainConfig,
    /// 65-byte uncompressed transaction public keys, in payload order.
    pub public_keys: PublicKeys,
}

impl StatelessInput {
    /// Serializes to schema-prefixed SSZ bytes, mirroring `serialize_stateless_input` in
    /// [`stateless_host.py`]. The fork is encoded into the schema identifier prefix.
    ///
    /// [`stateless_host.py`]: https://github.com/ethereum/execution-specs/blob/tests-zkevm@v0.6.2/src/ethereum/forks/amsterdam/stateless_host.py
    pub fn to_schema_prefixed_ssz(&self, fork: ProtocolFork) -> Vec<u8> {
        let mut out = Vec::with_capacity(STATELESS_INPUT_SCHEMA_ID_SIZE + self.encoded_len());
        let schema_id = ((fork.as_u64() as u16) << 8) | u16::from(STATELESS_INPUT_SCHEMA_REVISION);
        out.extend_from_slice(&schema_id.to_be_bytes());
        self.ssz_append(&mut out);
        out
    }

    /// Deserializes from schema-prefixed SSZ bytes, mirroring `deserialize_stateless_input` in
    /// [`stateless_guest.py`]. Returns the fork carried by the schema identifier alongside the
    /// decoded input, and rejects a payload request whose shape does not match that fork.
    ///
    /// [`stateless_guest.py`]: https://github.com/ethereum/execution-specs/blob/tests-zkevm@v0.6.2/src/ethereum/forks/amsterdam/stateless_guest.py
    pub fn from_schema_prefixed_ssz(bytes: &[u8]) -> Result<(ProtocolFork, Self), Error> {
        use ProtocolFork::*;
        let (schema_id, body) = bytes
            .split_first_chunk::<STATELESS_INPUT_SCHEMA_ID_SIZE>()
            .ok_or(Error::MissingSchemaId)?;
        let schema_id = u16::from_be_bytes(*schema_id);
        if (schema_id & 0xff) as u8 != STATELESS_INPUT_SCHEMA_REVISION {
            return Err(Error::UnsupportedSchemaId(schema_id));
        }
        let fork = ProtocolFork::from_u64(u64::from(schema_id >> 8))
            .ok_or(Error::UnsupportedSchemaId(schema_id))?;
        let input = match fork {
            Paris => StatelessInputBellatrix::from_ssz_bytes(body)?.into(),
            Shanghai => StatelessInputCapella::from_ssz_bytes(body)?.into(),
            Cancun => StatelessInputDeneb::from_ssz_bytes(body)?.into(),
            Prague | Osaka | BPO1 | BPO2 => StatelessInputElectraFulu::from_ssz_bytes(body)?.into(),
            Amsterdam => StatelessInputGloas::from_ssz_bytes(body)?.into(),
        };
        Ok((fork, input))
    }
}

macro_rules! declare_stateless_input_variants {
    ($($variant:ident),*) => {
        paste::paste! {
            $(
                #[derive(SszDecode)]
                struct [<StatelessInput $variant>] {
                    new_payload_request: new_payload_request::[<NewPayloadRequest $variant>],
                    witness: ExecutionWitness,
                    chain_config: ChainConfig,
                    public_keys: PublicKeys,
                }

                impl From<[<StatelessInput $variant>]> for StatelessInput {
                    fn from(input: [<StatelessInput $variant>]) -> StatelessInput {
                        StatelessInput {
                            new_payload_request: NewPayloadRequest::$variant(input.new_payload_request),
                            witness: input.witness,
                            chain_config: input.chain_config,
                            public_keys: input.public_keys,
                        }
                    }
                }
            )*
        }
    };
}

declare_stateless_input_variants!(Bellatrix, Capella, Deneb, ElectraFulu, Gloas);
