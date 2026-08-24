//! Canonical stateless validation input types.
//!
//! The types mirror [`stateless.py`] and their SSZ schemas in [`stateless_ssz.py`]. The wire
//! format is a 2-byte big-endian schema identifier followed by the SSZ-encoded `StatelessInput`
//! container.
//!
//! [`stateless.py`]: https://github.com/ethereum/execution-specs/blob/tests-zkevm@v0.8.2/src/ethereum/forks/amsterdam/stateless.py
//! [`stateless_ssz.py`]: https://github.com/ethereum/execution-specs/blob/tests-zkevm@v0.8.2/src/ethereum/forks/amsterdam/stateless_ssz.py

#![allow(missing_docs)]

use alloc::vec::Vec;
use libssz::{SszDecode, SszEncode};
use libssz_derive::{SszDecode, SszEncode};
use libssz_types::{ProgressiveList, SszList};

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
pub const MAX_WITNESS_HEADERS: usize = 256;
pub const MAX_BYTES_PER_WITNESS_NODE: usize = 1 << 10;
pub const MAX_BYTES_PER_CODE: usize = 1 << 16;
pub const MAX_BYTES_PER_HEADER: usize = 1 << 10;
pub const PUBLIC_KEY_BYTES: usize = 65;

/// Transaction public keys in payload order.
pub type PublicKeys = ProgressiveList<[u8; PUBLIC_KEY_BYTES]>;

/// Execution witness data for stateless validation.
#[derive(Debug, Clone, Default, PartialEq, Eq, SszEncode, SszDecode)]
pub struct ExecutionWitness {
    /// Hashed trie-node preimages needed during execution and state-root recomputation.
    pub state: ProgressiveList<SszList<u8, MAX_BYTES_PER_WITNESS_NODE>>,
    /// Contract-code preimages (created or accessed) needed during execution.
    pub codes: ProgressiveList<SszList<u8, MAX_BYTES_PER_CODE>>,
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

    /// Returns the complete schema identifier for this fork and the current revision.
    pub fn schema_id(self) -> u16 {
        ((self.as_u64() as u16) << 8) | u16::from(STATELESS_INPUT_SCHEMA_REVISION)
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
    /// Chain identifier used during payload validation and execution.
    pub chain_id: u64,
    /// 65-byte uncompressed transaction public keys, in payload order.
    pub public_keys: PublicKeys,
}

impl StatelessInput {
    /// Serializes to schema-prefixed SSZ bytes, mirroring `serialize_stateless_input` in
    /// [`stateless_host.py`]. The fork is encoded into the schema identifier prefix.
    ///
    /// [`stateless_host.py`]: https://github.com/ethereum/execution-specs/blob/tests-zkevm@v0.8.2/src/ethereum/forks/amsterdam/stateless_host.py
    pub fn to_schema_prefixed_ssz(&self, fork: ProtocolFork) -> Vec<u8> {
        let mut out = Vec::with_capacity(STATELESS_INPUT_SCHEMA_ID_SIZE + self.encoded_len());
        out.extend_from_slice(&fork.schema_id().to_be_bytes());
        self.ssz_append(&mut out);
        out
    }

    /// Deserializes from schema-prefixed SSZ bytes, mirroring `deserialize_stateless_input` in
    /// [`stateless_guest.py`]. Returns the fork carried by the schema identifier alongside the
    /// decoded input, and rejects a payload request whose shape does not match that fork.
    ///
    /// [`stateless_guest.py`]: https://github.com/ethereum/execution-specs/blob/tests-zkevm@v0.8.2/src/ethereum/forks/amsterdam/stateless_guest.py
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
                    chain_id: u64,
                    public_keys: PublicKeys,
                }

                impl From<[<StatelessInput $variant>]> for StatelessInput {
                    fn from(input: [<StatelessInput $variant>]) -> StatelessInput {
                        StatelessInput {
                            new_payload_request: NewPayloadRequest::$variant(input.new_payload_request),
                            witness: input.witness,
                            chain_id: input.chain_id,
                            public_keys: input.public_keys,
                        }
                    }
                }
            )*
        }
    };
}

declare_stateless_input_variants!(Bellatrix, Capella, Deneb, ElectraFulu, Gloas);

#[cfg(test)]
mod tests {
    use alloc::vec;

    use libssz::SszEncode as _;
    use libssz_merkle::{HashTreeRoot as _, Sha2Hasher};

    use super::*;
    use crate::guest::input::new_payload_request::{
        ExecutionPayloadV1, NewPayloadRequestBellatrix,
    };

    fn bellatrix_input() -> StatelessInput {
        StatelessInput {
            new_payload_request: NewPayloadRequest::Bellatrix(NewPayloadRequestBellatrix {
                execution_payload: ExecutionPayloadV1 {
                    parent_hash: [1; 32],
                    fee_recipient: [2; 20],
                    state_root: [3; 32],
                    receipts_root: [4; 32],
                    logs_bloom: [5; 256],
                    prev_randao: [6; 32],
                    block_number: 7,
                    gas_limit: 8,
                    gas_used: 9,
                    timestamp: 10,
                    extra_data: Default::default(),
                    base_fee_per_gas: [11; 32],
                    block_hash: [12; 32],
                    transactions: Default::default(),
                },
            }),
            witness: ExecutionWitness::default(),
            chain_id: 1,
            public_keys: Default::default(),
        }
    }

    #[test]
    fn schema_ids_cover_every_retained_fork() {
        let expected = [0x0e01, 0x0f01, 0x1001, 0x1101, 0x1201, 0x1301, 0x1401, 0x1501];
        for (fork_index, expected_schema_id) in
            (ProtocolFork::Paris.as_u64()..=ProtocolFork::Amsterdam.as_u64()).zip(expected)
        {
            assert_eq!(ProtocolFork::from_u64(fork_index).unwrap().schema_id(), expected_schema_id);
        }
    }

    #[test]
    fn rejects_truncated_trailing_and_unsupported_schema_inputs() {
        let bytes = bellatrix_input().to_schema_prefixed_ssz(ProtocolFork::Paris);
        assert_eq!(
            StatelessInput::from_schema_prefixed_ssz(&bytes).unwrap().0,
            ProtocolFork::Paris
        );

        assert!(StatelessInput::from_schema_prefixed_ssz(&bytes[..bytes.len() - 1]).is_err());
        let mut trailing = bytes.clone();
        trailing.push(0);
        assert!(StatelessInput::from_schema_prefixed_ssz(&trailing).is_err());

        let mut wrong_revision = bytes.clone();
        wrong_revision[1] = 2;
        assert!(matches!(
            StatelessInput::from_schema_prefixed_ssz(&wrong_revision),
            Err(Error::UnsupportedSchemaId(0x0e02))
        ));
        let mut wrong_fork = bytes;
        wrong_fork[0] = 0x0d;
        assert!(matches!(
            StatelessInput::from_schema_prefixed_ssz(&wrong_fork),
            Err(Error::UnsupportedSchemaId(0x0d01))
        ));
    }

    #[test]
    fn progressive_witness_and_public_key_roots_match_reference_vectors() {
        let hasher = Sha2Hasher;
        let state: ProgressiveList<SszList<u8, MAX_BYTES_PER_WITNESS_NODE>> =
            ProgressiveList::from(vec![vec![0xaa_u8; 3].try_into().unwrap()]);
        let codes: ProgressiveList<SszList<u8, MAX_BYTES_PER_CODE>> =
            ProgressiveList::from(vec![vec![0xbb_u8; 5].try_into().unwrap()]);
        let public_keys = PublicKeys::from(vec![[0xcc; PUBLIC_KEY_BYTES]]);

        assert_eq!(
            const_hex::encode(state.hash_tree_root(&hasher)),
            "8a25ed609796eeb1a5d772d19d1e9236b069e0dc2c9fc1f5db166e73c0e7d5fc"
        );
        assert_eq!(
            const_hex::encode(codes.hash_tree_root(&hasher)),
            "a547789a596b86c52d812f5e04a9375ad0030c486748e04eb80ae80fc7a93e9c"
        );
        assert_eq!(
            const_hex::encode(public_keys.hash_tree_root(&hasher)),
            "4c4b20e8ab80e7c80dc192092e936a59dd856cb8fa3f7e73b1e05e552fa0f783"
        );

        let bounded: SszList<[u8; PUBLIC_KEY_BYTES], 8> =
            vec![[0xcc; PUBLIC_KEY_BYTES]].try_into().unwrap();
        assert_eq!(public_keys.to_ssz(), bounded.to_ssz());
    }
}
