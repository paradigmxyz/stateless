//! New payload request types and their dependencies, mirroring [`types.py`], [`requests.py`],
//! and [`blocks.py`].
//!
//! The execution payload containers keep the V1 to V4 names defined by the engine API in
//! execution-apis, because a multi-fork crate needs distinct names while each execution-specs
//! fork module defines a single `ExecutionPayload` shape.
//!
//! [`types.py`]: https://github.com/ethereum/execution-specs/blob/tests-zkevm@v0.6.2/src/ethereum/forks/amsterdam/execution_engine/types.py
//! [`requests.py`]: https://github.com/ethereum/execution-specs/blob/tests-zkevm@v0.6.2/src/ethereum/forks/amsterdam/execution_engine/requests.py
//! [`blocks.py`]: https://github.com/ethereum/execution-specs/blob/tests-zkevm@v0.6.2/src/ethereum/forks/amsterdam/blocks.py

#![allow(missing_docs)]

use alloc::vec::Vec;

use libssz::{DecodeError, SszDecode, SszEncode};
use libssz_derive::{HashTreeRoot, SszDecode, SszEncode};
use libssz_merkle::{HashTreeRoot, Sha256Hasher};
use libssz_types::SszList;

/// Primitive types from the Amsterdam stateless schema.
pub type Hash32 = [u8; 32];
pub type Bytes48 = [u8; 48];
pub type Bytes96 = [u8; 96];
pub type Address = [u8; 20];
pub type Uint256Bytes = [u8; 32];
pub type Bloom = [u8; 256];
pub type VersionedHash = Hash32;
pub type ExtraData = SszList<u8, MAX_EXTRA_DATA_BYTES>;

/// SSZ list bounds from the Amsterdam stateless schema.
pub const MAX_EXTRA_DATA_BYTES: usize = 32;
pub const MAX_WITHDRAWALS_PER_PAYLOAD: usize = 16;
pub const MAX_TRANSACTIONS_PER_PAYLOAD: usize = 1 << 20;
pub const MAX_BYTES_PER_TRANSACTION: usize = 1 << 30;
pub const MAX_BLOB_COMMITMENTS_PER_BLOCK: usize = 4096;
pub const MAX_DEPOSIT_REQUESTS_PER_PAYLOAD: usize = 8192;
pub const MAX_WITHDRAWAL_REQUESTS_PER_PAYLOAD: usize = 16;
pub const MAX_CONSOLIDATION_REQUESTS_PER_PAYLOAD: usize = 2;
pub const MAX_BUILDER_DEPOSIT_REQUESTS_PER_PAYLOAD: usize = 64;
pub const MAX_BUILDER_EXIT_REQUESTS_PER_PAYLOAD: usize = 16;

/// Composite types from the Amsterdam stateless schema.
pub type BlockAccessList = SszList<u8, MAX_BYTES_PER_TRANSACTION>;
pub type Transaction = SszList<u8, MAX_BYTES_PER_TRANSACTION>;
pub type Transactions = SszList<Transaction, MAX_TRANSACTIONS_PER_PAYLOAD>;
pub type Withdrawals = SszList<Withdrawal, MAX_WITHDRAWALS_PER_PAYLOAD>;
pub type VersionedHashes = SszList<VersionedHash, MAX_BLOB_COMMITMENTS_PER_BLOCK>;
pub type DepositRequests = SszList<DepositRequest, MAX_DEPOSIT_REQUESTS_PER_PAYLOAD>;
pub type WithdrawalRequests = SszList<WithdrawalRequest, MAX_WITHDRAWAL_REQUESTS_PER_PAYLOAD>;
pub type ConsolidationRequests =
    SszList<ConsolidationRequest, MAX_CONSOLIDATION_REQUESTS_PER_PAYLOAD>;
pub type BuilderDepositRequests =
    SszList<BuilderDepositRequest, MAX_BUILDER_DEPOSIT_REQUESTS_PER_PAYLOAD>;
pub type BuilderExitRequests = SszList<BuilderExitRequest, MAX_BUILDER_EXIT_REQUESTS_PER_PAYLOAD>;

/// Withdrawals represent a transfer of ETH from the consensus layer (beacon chain) to the
/// execution layer, as validated by the consensus layer. Each withdrawal is listed in the block's
/// list of withdrawals.
#[derive(Debug, Clone, PartialEq, Eq, HashTreeRoot, SszEncode, SszDecode)]
pub struct Withdrawal {
    /// The unique index of the withdrawal, incremented for each withdrawal processed.
    pub index: u64,
    /// The index of the validator on the consensus layer that is withdrawing.
    pub validator_index: u64,
    /// The execution-layer address receiving the withdrawn ETH.
    pub address: Address,
    /// The amount of ETH being withdrawn.
    pub amount: u64,
}

/// A single EIP-6110 deposit request.
#[derive(Debug, Clone, PartialEq, Eq, HashTreeRoot, SszEncode, SszDecode)]
pub struct DepositRequest {
    pub pubkey: Bytes48,
    pub withdrawal_credentials: Hash32,
    pub amount: u64,
    pub signature: Bytes96,
    pub index: u64,
}

/// A single EIP-7002 withdrawal request.
#[derive(Debug, Clone, PartialEq, Eq, HashTreeRoot, SszEncode, SszDecode)]
pub struct WithdrawalRequest {
    pub source_address: Address,
    pub validator_pubkey: Bytes48,
    pub amount: u64,
}

/// A single EIP-7251 consolidation request.
#[derive(Debug, Clone, PartialEq, Eq, HashTreeRoot, SszEncode, SszDecode)]
pub struct ConsolidationRequest {
    pub source_address: Address,
    pub source_pubkey: Bytes48,
    pub target_pubkey: Bytes48,
}

/// A single EIP-8282 builder deposit request.
#[derive(Debug, Clone, PartialEq, Eq, HashTreeRoot, SszEncode, SszDecode)]
pub struct BuilderDepositRequest {
    pub pubkey: Bytes48,
    pub withdrawal_credentials: Hash32,
    pub amount: u64,
    pub signature: Bytes96,
}

/// A single EIP-8282 builder exit request.
#[derive(Debug, Clone, PartialEq, Eq, HashTreeRoot, SszEncode, SszDecode)]
pub struct BuilderExitRequest {
    pub source_address: Address,
    pub pubkey: Bytes48,
}

/// Typed engine-API container of execution-layer triggered requests, as of Electra.
///
/// Mirrors the consensus-layer `ExecutionRequests` Container.
#[derive(Debug, Clone, Default, PartialEq, Eq, HashTreeRoot, SszEncode, SszDecode)]
pub struct ExecutionRequestsElectraFulu {
    pub deposits: DepositRequests,
    pub withdrawals: WithdrawalRequests,
    pub consolidations: ConsolidationRequests,
}

/// Typed engine-API container of execution-layer triggered requests, as of Gloas, which
/// EIP-8282 extends with the builder deposit and builder exit lists.
#[derive(Debug, Clone, Default, PartialEq, Eq, HashTreeRoot, SszEncode, SszDecode)]
pub struct ExecutionRequestsGloas {
    pub deposits: DepositRequests,
    pub withdrawals: WithdrawalRequests,
    pub consolidations: ConsolidationRequests,
    pub builder_deposits: BuilderDepositRequests,
    pub builder_exits: BuilderExitRequests,
}

#[derive(Debug, Clone, PartialEq, Eq, HashTreeRoot, SszEncode, SszDecode)]
pub struct ExecutionPayloadV1 {
    pub parent_hash: Hash32,
    pub fee_recipient: Address,
    pub state_root: Hash32,
    pub receipts_root: Hash32,
    pub logs_bloom: Bloom,
    pub prev_randao: Hash32,
    pub block_number: u64,
    pub gas_limit: u64,
    pub gas_used: u64,
    pub timestamp: u64,
    pub extra_data: ExtraData,
    pub base_fee_per_gas: Uint256Bytes,
    pub block_hash: Hash32,
    pub transactions: Transactions,
}

#[derive(Debug, Clone, PartialEq, Eq, HashTreeRoot, SszEncode, SszDecode)]
pub struct ExecutionPayloadV2 {
    pub parent_hash: Hash32,
    pub fee_recipient: Address,
    pub state_root: Hash32,
    pub receipts_root: Hash32,
    pub logs_bloom: Bloom,
    pub prev_randao: Hash32,
    pub block_number: u64,
    pub gas_limit: u64,
    pub gas_used: u64,
    pub timestamp: u64,
    pub extra_data: ExtraData,
    pub base_fee_per_gas: Uint256Bytes,
    pub block_hash: Hash32,
    pub transactions: Transactions,
    pub withdrawals: Withdrawals,
}

#[derive(Debug, Clone, PartialEq, Eq, HashTreeRoot, SszEncode, SszDecode)]
pub struct ExecutionPayloadV3 {
    pub parent_hash: Hash32,
    pub fee_recipient: Address,
    pub state_root: Hash32,
    pub receipts_root: Hash32,
    pub logs_bloom: Bloom,
    pub prev_randao: Hash32,
    pub block_number: u64,
    pub gas_limit: u64,
    pub gas_used: u64,
    pub timestamp: u64,
    pub extra_data: ExtraData,
    pub base_fee_per_gas: Uint256Bytes,
    pub block_hash: Hash32,
    pub transactions: Transactions,
    pub withdrawals: Withdrawals,
    pub blob_gas_used: u64,
    pub excess_blob_gas: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, HashTreeRoot, SszEncode, SszDecode)]
pub struct ExecutionPayloadV4 {
    pub parent_hash: Hash32,
    pub fee_recipient: Address,
    pub state_root: Hash32,
    pub receipts_root: Hash32,
    pub logs_bloom: Bloom,
    pub prev_randao: Hash32,
    pub block_number: u64,
    pub gas_limit: u64,
    pub gas_used: u64,
    pub timestamp: u64,
    pub extra_data: ExtraData,
    pub base_fee_per_gas: Uint256Bytes,
    pub block_hash: Hash32,
    pub transactions: Transactions,
    pub withdrawals: Withdrawals,
    pub blob_gas_used: u64,
    pub excess_blob_gas: u64,
    pub block_access_list: BlockAccessList,
    pub slot_number: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, HashTreeRoot, SszEncode, SszDecode)]
pub struct NewPayloadRequestBellatrix {
    pub execution_payload: ExecutionPayloadV1,
}

#[derive(Debug, Clone, PartialEq, Eq, HashTreeRoot, SszEncode, SszDecode)]
pub struct NewPayloadRequestCapella {
    pub execution_payload: ExecutionPayloadV2,
}

#[derive(Debug, Clone, PartialEq, Eq, HashTreeRoot, SszEncode, SszDecode)]
pub struct NewPayloadRequestDeneb {
    pub execution_payload: ExecutionPayloadV3,
    pub versioned_hashes: VersionedHashes,
    pub parent_beacon_block_root: Hash32,
}

#[derive(Debug, Clone, PartialEq, Eq, HashTreeRoot, SszEncode, SszDecode)]
pub struct NewPayloadRequestElectraFulu {
    pub execution_payload: ExecutionPayloadV3,
    pub versioned_hashes: VersionedHashes,
    pub parent_beacon_block_root: Hash32,
    pub execution_requests: ExecutionRequestsElectraFulu,
}

#[derive(Debug, Clone, PartialEq, Eq, HashTreeRoot, SszEncode, SszDecode)]
pub struct NewPayloadRequestGloas {
    pub execution_payload: ExecutionPayloadV4,
    pub versioned_hashes: VersionedHashes,
    pub parent_beacon_block_root: Hash32,
    pub execution_requests: ExecutionRequestsGloas,
}

/// Consensus-layer new payload request with one container shape per fork.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NewPayloadRequest {
    Bellatrix(NewPayloadRequestBellatrix),
    Capella(NewPayloadRequestCapella),
    Deneb(NewPayloadRequestDeneb),
    ElectraFulu(NewPayloadRequestElectraFulu),
    Gloas(NewPayloadRequestGloas),
}

impl NewPayloadRequest {
    /// Returns the block number of the execution payload.
    pub fn block_number(&self) -> u64 {
        match self {
            Self::Bellatrix(request) => request.execution_payload.block_number,
            Self::Capella(request) => request.execution_payload.block_number,
            Self::Deneb(request) => request.execution_payload.block_number,
            Self::ElectraFulu(request) => request.execution_payload.block_number,
            Self::Gloas(request) => request.execution_payload.block_number,
        }
    }

    /// Returns the timestamp of the execution payload.
    pub fn timestamp(&self) -> u64 {
        match self {
            Self::Bellatrix(request) => request.execution_payload.timestamp,
            Self::Capella(request) => request.execution_payload.timestamp,
            Self::Deneb(request) => request.execution_payload.timestamp,
            Self::ElectraFulu(request) => request.execution_payload.timestamp,
            Self::Gloas(request) => request.execution_payload.timestamp,
        }
    }

    /// Returns the gas used by the execution payload.
    pub fn gas_used(&self) -> u64 {
        match self {
            Self::Bellatrix(request) => request.execution_payload.gas_used,
            Self::Capella(request) => request.execution_payload.gas_used,
            Self::Deneb(request) => request.execution_payload.gas_used,
            Self::ElectraFulu(request) => request.execution_payload.gas_used,
            Self::Gloas(request) => request.execution_payload.gas_used,
        }
    }

    /// Returns the block hash of the execution payload.
    pub fn block_hash(&self) -> Hash32 {
        match self {
            Self::Bellatrix(request) => request.execution_payload.block_hash,
            Self::Capella(request) => request.execution_payload.block_hash,
            Self::Deneb(request) => request.execution_payload.block_hash,
            Self::ElectraFulu(request) => request.execution_payload.block_hash,
            Self::Gloas(request) => request.execution_payload.block_hash,
        }
    }

    /// Returns the transactions of the execution payload.
    pub fn transactions(&self) -> &Transactions {
        match self {
            Self::Bellatrix(request) => &request.execution_payload.transactions,
            Self::Capella(request) => &request.execution_payload.transactions,
            Self::Deneb(request) => &request.execution_payload.transactions,
            Self::ElectraFulu(request) => &request.execution_payload.transactions,
            Self::Gloas(request) => &request.execution_payload.transactions,
        }
    }
}

impl HashTreeRoot for NewPayloadRequest {
    fn hash_tree_root(&self, hasher: &impl Sha256Hasher) -> [u8; 32] {
        match self {
            Self::Bellatrix(request) => request.hash_tree_root(hasher),
            Self::Capella(request) => request.hash_tree_root(hasher),
            Self::Deneb(request) => request.hash_tree_root(hasher),
            Self::ElectraFulu(request) => request.hash_tree_root(hasher),
            Self::Gloas(request) => request.hash_tree_root(hasher),
        }
    }
}

impl SszEncode for NewPayloadRequest {
    fn is_fixed_size() -> bool {
        false
    }

    fn fixed_size() -> usize {
        0
    }

    fn encoded_len(&self) -> usize {
        match self {
            Self::Bellatrix(request) => request.encoded_len(),
            Self::Capella(request) => request.encoded_len(),
            Self::Deneb(request) => request.encoded_len(),
            Self::ElectraFulu(request) => request.encoded_len(),
            Self::Gloas(request) => request.encoded_len(),
        }
    }

    fn ssz_append(&self, buf: &mut Vec<u8>) {
        match self {
            Self::Bellatrix(request) => request.ssz_append(buf),
            Self::Capella(request) => request.ssz_append(buf),
            Self::Deneb(request) => request.ssz_append(buf),
            Self::ElectraFulu(request) => request.ssz_append(buf),
            Self::Gloas(request) => request.ssz_append(buf),
        }
    }
}

impl SszDecode for NewPayloadRequest {
    fn is_fixed_size() -> bool {
        false
    }

    fn fixed_size() -> usize {
        0
    }

    /// Decodes by attempting each container shape from the newest fork to the
    /// oldest and returning the first success.
    fn from_ssz_bytes(bytes: &[u8]) -> Result<Self, DecodeError> {
        NewPayloadRequestGloas::from_ssz_bytes(bytes)
            .map(Self::Gloas)
            .or_else(|_| NewPayloadRequestElectraFulu::from_ssz_bytes(bytes).map(Self::ElectraFulu))
            .or_else(|_| NewPayloadRequestDeneb::from_ssz_bytes(bytes).map(Self::Deneb))
            .or_else(|_| NewPayloadRequestCapella::from_ssz_bytes(bytes).map(Self::Capella))
            .or_else(|_| NewPayloadRequestBellatrix::from_ssz_bytes(bytes).map(Self::Bellatrix))
    }
}

#[cfg(test)]
mod tests {
    use alloc::vec;

    use libssz_merkle::Sha2Hasher;

    use crate::guest::input::{
        ChainConfig, ExecutionWitness, ProtocolFork, StatelessInput, new_payload_request::*,
    };

    fn payload_v1() -> ExecutionPayloadV1 {
        ExecutionPayloadV1 {
            parent_hash: [1; 32],
            fee_recipient: [2; 20],
            state_root: [3; 32],
            receipts_root: [4; 32],
            logs_bloom: [5; 256],
            prev_randao: [6; 32],
            block_number: 100,
            gas_limit: 30_000_000,
            gas_used: 21_000,
            timestamp: 1_700_000_000,
            extra_data: vec![0xee; 7].try_into().unwrap(),
            base_fee_per_gas: [7; 32],
            block_hash: [8; 32],
            transactions: vec![vec![0xdd_u8; 8].try_into().unwrap()].try_into().unwrap(),
        }
    }

    fn payload_v2() -> ExecutionPayloadV2 {
        let v1 = payload_v1();
        ExecutionPayloadV2 {
            parent_hash: v1.parent_hash,
            fee_recipient: v1.fee_recipient,
            state_root: v1.state_root,
            receipts_root: v1.receipts_root,
            logs_bloom: v1.logs_bloom,
            prev_randao: v1.prev_randao,
            block_number: v1.block_number,
            gas_limit: v1.gas_limit,
            gas_used: v1.gas_used,
            timestamp: v1.timestamp,
            extra_data: v1.extra_data,
            base_fee_per_gas: v1.base_fee_per_gas,
            block_hash: v1.block_hash,
            transactions: v1.transactions,
            withdrawals: vec![Withdrawal {
                index: 1,
                validator_index: 2,
                address: [3; 20],
                amount: 4,
            }]
            .try_into()
            .unwrap(),
        }
    }

    fn payload_v3() -> ExecutionPayloadV3 {
        let v2 = payload_v2();
        ExecutionPayloadV3 {
            parent_hash: v2.parent_hash,
            fee_recipient: v2.fee_recipient,
            state_root: v2.state_root,
            receipts_root: v2.receipts_root,
            logs_bloom: v2.logs_bloom,
            prev_randao: v2.prev_randao,
            block_number: v2.block_number,
            gas_limit: v2.gas_limit,
            gas_used: v2.gas_used,
            timestamp: v2.timestamp,
            extra_data: v2.extra_data,
            base_fee_per_gas: v2.base_fee_per_gas,
            block_hash: v2.block_hash,
            transactions: v2.transactions,
            withdrawals: v2.withdrawals,
            blob_gas_used: 131_072,
            excess_blob_gas: 262_144,
        }
    }

    fn payload_v4() -> ExecutionPayloadV4 {
        let v3 = payload_v3();
        ExecutionPayloadV4 {
            parent_hash: v3.parent_hash,
            fee_recipient: v3.fee_recipient,
            state_root: v3.state_root,
            receipts_root: v3.receipts_root,
            logs_bloom: v3.logs_bloom,
            prev_randao: v3.prev_randao,
            block_number: v3.block_number,
            gas_limit: v3.gas_limit,
            gas_used: v3.gas_used,
            timestamp: v3.timestamp,
            extra_data: v3.extra_data,
            base_fee_per_gas: v3.base_fee_per_gas,
            block_hash: v3.block_hash,
            transactions: v3.transactions,
            withdrawals: v3.withdrawals,
            blob_gas_used: v3.blob_gas_used,
            excess_blob_gas: v3.excess_blob_gas,
            block_access_list: vec![0xba; 33].try_into().unwrap(),
            slot_number: 42,
        }
    }

    fn versioned_hashes() -> VersionedHashes {
        vec![[9; 32]].try_into().unwrap()
    }

    fn deposit_requests() -> DepositRequests {
        vec![DepositRequest {
            pubkey: [1; 48],
            withdrawal_credentials: [2; 32],
            amount: 3,
            signature: [4; 96],
            index: 5,
        }]
        .try_into()
        .unwrap()
    }

    fn withdrawal_requests() -> WithdrawalRequests {
        vec![WithdrawalRequest { source_address: [1; 20], validator_pubkey: [2; 48], amount: 3 }]
            .try_into()
            .unwrap()
    }

    fn consolidation_requests() -> ConsolidationRequests {
        vec![ConsolidationRequest {
            source_address: [1; 20],
            source_pubkey: [2; 48],
            target_pubkey: [3; 48],
        }]
        .try_into()
        .unwrap()
    }

    fn builder_deposit_requests() -> BuilderDepositRequests {
        vec![BuilderDepositRequest {
            pubkey: [1; 48],
            withdrawal_credentials: [2; 32],
            amount: 3,
            signature: [4; 96],
        }]
        .try_into()
        .unwrap()
    }

    fn builder_exit_requests() -> BuilderExitRequests {
        vec![BuilderExitRequest { source_address: [1; 20], pubkey: [2; 48] }].try_into().unwrap()
    }

    fn execution_requests_electra_fulu() -> ExecutionRequestsElectraFulu {
        ExecutionRequestsElectraFulu {
            deposits: deposit_requests(),
            withdrawals: withdrawal_requests(),
            consolidations: consolidation_requests(),
        }
    }

    fn execution_requests_gloas() -> ExecutionRequestsGloas {
        ExecutionRequestsGloas {
            deposits: deposit_requests(),
            withdrawals: withdrawal_requests(),
            consolidations: consolidation_requests(),
            builder_deposits: builder_deposit_requests(),
            builder_exits: builder_exit_requests(),
        }
    }

    fn bellatrix() -> NewPayloadRequest {
        NewPayloadRequest::Bellatrix(NewPayloadRequestBellatrix { execution_payload: payload_v1() })
    }

    fn capella() -> NewPayloadRequest {
        NewPayloadRequest::Capella(NewPayloadRequestCapella { execution_payload: payload_v2() })
    }

    fn deneb() -> NewPayloadRequest {
        NewPayloadRequest::Deneb(NewPayloadRequestDeneb {
            execution_payload: payload_v3(),
            versioned_hashes: versioned_hashes(),
            parent_beacon_block_root: [10; 32],
        })
    }

    fn electra_fulu() -> NewPayloadRequest {
        NewPayloadRequest::ElectraFulu(NewPayloadRequestElectraFulu {
            execution_payload: payload_v3(),
            versioned_hashes: versioned_hashes(),
            parent_beacon_block_root: [10; 32],
            execution_requests: execution_requests_electra_fulu(),
        })
    }

    fn gloas() -> NewPayloadRequest {
        NewPayloadRequest::Gloas(NewPayloadRequestGloas {
            execution_payload: payload_v4(),
            versioned_hashes: versioned_hashes(),
            parent_beacon_block_root: [10; 32],
            execution_requests: execution_requests_gloas(),
        })
    }

    fn stateless_input(new_payload_request: NewPayloadRequest) -> StatelessInput {
        StatelessInput {
            new_payload_request,
            witness: ExecutionWitness::default(),
            chain_config: ChainConfig::default(),
            public_keys: Default::default(),
        }
    }

    #[test]
    fn from_ssz_bytes_decodes_every_variant() {
        for request in [bellatrix(), capella(), deneb(), electra_fulu(), gloas()] {
            let decoded = NewPayloadRequest::from_ssz_bytes(&request.to_ssz()).unwrap();
            assert_eq!(decoded, request);
            assert_eq!(decoded.hash_tree_root(&Sha2Hasher), request.hash_tree_root(&Sha2Hasher));
        }
    }

    #[test]
    fn matches_fork_partitions_every_variant_and_fork() {
        const ELECTRA_FULU_FORKS: &[ProtocolFork] =
            &[ProtocolFork::Prague, ProtocolFork::Osaka, ProtocolFork::BPO1, ProtocolFork::BPO2];
        for (request, matching) in [
            (bellatrix(), [ProtocolFork::Paris].as_slice()),
            (capella(), [ProtocolFork::Shanghai].as_slice()),
            (deneb(), [ProtocolFork::Cancun].as_slice()),
            (electra_fulu(), ELECTRA_FULU_FORKS),
            (gloas(), [ProtocolFork::Amsterdam].as_slice()),
        ] {
            for fork in 1..=ProtocolFork::Amsterdam.as_u64() {
                let fork = ProtocolFork::from_u64(fork).unwrap();
                let input = stateless_input(request.clone());
                let result =
                    StatelessInput::from_schema_prefixed_ssz(&input.to_schema_prefixed_ssz(fork));
                if matching.contains(&fork) {
                    let (decoded_fork, decoded) = result.unwrap();
                    assert_eq!(decoded_fork, fork);
                    assert_eq!(decoded.new_payload_request, request);
                } else {
                    assert!(result.is_err());
                }
            }
        }
    }
}
