//! Conversions from the canonical stateless input to the reth structures.

use alloc::{
    collections::BTreeMap,
    string::{String, ToString},
    sync::Arc,
    vec::Vec,
};
use core::cmp::Ordering;

use alloy_consensus::Block;
use alloy_eips::eip7840::BlobParams;
use alloy_primitives::{Address, B256, Bloom, Bytes, U256};
use alloy_rpc_types_engine::{
    CancunPayloadFields, ExecutionData, ExecutionPayloadSidecar, PayloadError,
};
use reth_chainspec::{ChainSpec, EthereumHardforks};
use reth_evm_ethereum::EthEvmConfig;
use reth_payload_validator::{cancun, prague, shanghai};
use reth_primitives_traits::{Block as _, SealedBlock, SignedTransaction};
use stateless::{Genesis, UncompressedPublicKey};
use stateless_validator_common::{
    Sha256Hasher, SszEncode, SszList,
    guest::{
        Error as CommonError,
        input::{
            ChainConfig, ExecutionWitness, ProtocolFork, StatelessInput,
            new_payload_request::{
                ExecutionPayloadV1, ExecutionPayloadV2, ExecutionPayloadV3, ExecutionPayloadV4,
                ExecutionRequestsElectraFulu, ExecutionRequestsGloas, Hash32, NewPayloadRequest,
                VersionedHashes, Withdrawals,
            },
        },
    },
};

use crate::guest::{crypto, error::Error};

/// EIP-7685 request type prefixes.
const DEPOSIT_REQUEST_TYPE: u8 = 0x00;
const WITHDRAWAL_REQUEST_TYPE: u8 = 0x01;
const CONSOLIDATION_REQUEST_TYPE: u8 = 0x02;
const BUILDER_DEPOSIT_REQUEST_TYPE: u8 = 0x03;
const BUILDER_EXIT_REQUEST_TYPE: u8 = 0x04;

/// Reconstructed reth validation input consumed by `stateless_validation_with_trie`.
pub(crate) struct RethInput {
    pub(crate) chain_spec: Arc<ChainSpec>,
    pub(crate) evm_config: EthEvmConfig,
    pub(crate) block: Block<reth_ethereum_primitives::TransactionSigned>,
    pub(crate) witness: stateless::ExecutionWitness,
    pub(crate) public_keys: Vec<UncompressedPublicKey>,
}

/// Converts the decoded canonical stateless input into the reth validation
/// input consumed by `stateless_validation_with_trie`.
pub(crate) fn to_reth_input(fork: ProtocolFork, input: StatelessInput) -> Result<RethInput, Error> {
    let chain_spec = Arc::new(ChainSpec::from(Genesis {
        config: to_reth_chain_config(fork, &input.chain_config)?,
        ..Default::default()
    }));
    let evm_config = EthEvmConfig::new(chain_spec.clone());
    let block = to_reth_block(input.new_payload_request, chain_spec.clone())?.into_block();
    let witness = to_reth_witness(input.witness);
    let public_keys = Vec::from_iter(input.public_keys.into_iter().map(UncompressedPublicKey));
    Ok(RethInput { chain_spec, evm_config, block, witness, public_keys })
}

/// Converts a chain configuration into an [`alloy_genesis::ChainConfig`].
fn to_reth_chain_config(
    fork: ProtocolFork,
    config: &ChainConfig,
) -> Result<alloy_genesis::ChainConfig, Error> {
    let (activation_block_number, activation_timestamp) = if fork >= ProtocolFork::Shanghai {
        let timestamp =
            config.active_fork.activation.timestamp().ok_or(CommonError::InvalidForkActivation)?;
        (0, timestamp)
    } else {
        let block_number = config
            .active_fork
            .activation
            .block_number()
            .ok_or(CommonError::InvalidForkActivation)?;
        (block_number, 0)
    };
    let block_at = |target| match fork.cmp(&target) {
        Ordering::Greater => Some(0),
        Ordering::Equal => Some(activation_block_number),
        Ordering::Less => None,
    };
    let time_at = |target| match fork.cmp(&target) {
        Ordering::Greater => Some(0),
        Ordering::Equal => Some(activation_timestamp),
        Ordering::Less => None,
    };

    Ok(alloy_genesis::ChainConfig {
        chain_id: config.chain_id,
        homestead_block: block_at(ProtocolFork::Homestead),
        dao_fork_block: block_at(ProtocolFork::DAOFork),
        dao_fork_support: fork >= ProtocolFork::DAOFork,
        eip150_block: block_at(ProtocolFork::TangerineWhistle),
        eip155_block: block_at(ProtocolFork::SpuriousDragon),
        eip158_block: block_at(ProtocolFork::SpuriousDragon),
        byzantium_block: block_at(ProtocolFork::Byzantium),
        constantinople_block: block_at(ProtocolFork::StPetersburg),
        petersburg_block: block_at(ProtocolFork::StPetersburg),
        istanbul_block: block_at(ProtocolFork::Istanbul),
        muir_glacier_block: block_at(ProtocolFork::MuirGlacier),
        berlin_block: block_at(ProtocolFork::Berlin),
        london_block: block_at(ProtocolFork::London),
        arrow_glacier_block: block_at(ProtocolFork::ArrowGlacier),
        gray_glacier_block: block_at(ProtocolFork::GrayGlacier),
        merge_netsplit_block: block_at(ProtocolFork::Paris),
        shanghai_time: time_at(ProtocolFork::Shanghai),
        cancun_time: time_at(ProtocolFork::Cancun),
        prague_time: time_at(ProtocolFork::Prague),
        osaka_time: time_at(ProtocolFork::Osaka),
        bpo1_time: time_at(ProtocolFork::BPO1),
        bpo2_time: time_at(ProtocolFork::BPO2),
        bpo3_time: None,
        bpo4_time: None,
        bpo5_time: None,
        amsterdam_time: time_at(ProtocolFork::Amsterdam),
        terminal_total_difficulty: (fork >= ProtocolFork::Paris).then_some(U256::ZERO),
        terminal_total_difficulty_passed: fork >= ProtocolFork::Paris,
        blob_schedule: active_fork_blob_schedule(fork),
        deposit_contract_address: Some(alloy_eips::eip6110::MAINNET_DEPOSIT_CONTRACT_ADDRESS),
        ..Default::default()
    })
}

/// Builds a reth blob schedule for the active fork.
fn active_fork_blob_schedule(fork: ProtocolFork) -> BTreeMap<String, BlobParams> {
    let (key, params) = match fork {
        ProtocolFork::Cancun => ("cancun", BlobParams::cancun()),
        ProtocolFork::Prague => ("prague", BlobParams::prague()),
        ProtocolFork::Osaka => ("osaka", BlobParams::osaka()),
        ProtocolFork::BPO1 => ("bpo1", BlobParams::bpo1()),
        ProtocolFork::BPO2 => ("bpo2", BlobParams::bpo2()),
        // The amsterdam arm in `blob_schedule_blob_params` of alloy-genesis is
        // spelled `Amsterdam` while every other fork key is lowercase.
        ProtocolFork::Amsterdam => ("Amsterdam", BlobParams::bpo2()),
        _ => return BTreeMap::new(),
    };
    BTreeMap::from([(key.to_string(), params)])
}

/// Converts the new payload request into engine-API execution data.
fn new_payload_request_to_execution_data(request: NewPayloadRequest) -> ExecutionData {
    let hasher = crypto::sha256_hasher();
    match request {
        NewPayloadRequest::Bellatrix(request) => ExecutionData::new(
            alloy_rpc_types_engine::ExecutionPayload::V1(to_alloy_payload_v1(
                request.execution_payload,
            )),
            ExecutionPayloadSidecar::none(),
        ),
        NewPayloadRequest::Capella(request) => ExecutionData::new(
            alloy_rpc_types_engine::ExecutionPayload::V2(to_alloy_payload_v2(
                request.execution_payload,
            )),
            ExecutionPayloadSidecar::none(),
        ),
        NewPayloadRequest::Deneb(request) => ExecutionData::new(
            alloy_rpc_types_engine::ExecutionPayload::V3(to_alloy_payload_v3(
                request.execution_payload,
            )),
            ExecutionPayloadSidecar::v3(cancun_fields(
                request.versioned_hashes,
                request.parent_beacon_block_root,
            )),
        ),
        NewPayloadRequest::ElectraFulu(request) => ExecutionData::new(
            alloy_rpc_types_engine::ExecutionPayload::V3(to_alloy_payload_v3(
                request.execution_payload,
            )),
            ExecutionPayloadSidecar::v4(
                cancun_fields(request.versioned_hashes, request.parent_beacon_block_root),
                prague_fields(compute_requests_hash_electra_fulu(
                    &request.execution_requests,
                    &hasher,
                )),
            ),
        ),
        NewPayloadRequest::Gloas(request) => ExecutionData::new(
            alloy_rpc_types_engine::ExecutionPayload::V4(to_alloy_payload_v4(
                request.execution_payload,
            )),
            ExecutionPayloadSidecar::v4(
                cancun_fields(request.versioned_hashes, request.parent_beacon_block_root),
                prague_fields(compute_requests_hash_gloas(&request.execution_requests, &hasher)),
            ),
        ),
    }
}

/// Builds the alloy V1 payload from any canonical payload version, which all
/// share the V1 field subset.
macro_rules! to_alloy_payload_v1 {
    ($payload:expr) => {
        alloy_rpc_types_engine::ExecutionPayloadV1 {
            parent_hash: B256::from($payload.parent_hash),
            fee_recipient: Address::from($payload.fee_recipient),
            state_root: B256::from($payload.state_root),
            receipts_root: B256::from($payload.receipts_root),
            logs_bloom: Bloom::from_slice(&$payload.logs_bloom[..]),
            prev_randao: B256::from($payload.prev_randao),
            block_number: $payload.block_number,
            gas_limit: $payload.gas_limit,
            gas_used: $payload.gas_used,
            timestamp: $payload.timestamp,
            extra_data: Bytes::from($payload.extra_data.into_inner()),
            base_fee_per_gas: U256::from_le_bytes($payload.base_fee_per_gas),
            block_hash: B256::from($payload.block_hash),
            transactions: $payload
                .transactions
                .into_iter()
                .map(|tx| Bytes::from(tx.into_inner()))
                .collect(),
        }
    };
}

fn to_alloy_payload_v1(payload: ExecutionPayloadV1) -> alloy_rpc_types_engine::ExecutionPayloadV1 {
    to_alloy_payload_v1!(payload)
}

fn to_alloy_payload_v2(payload: ExecutionPayloadV2) -> alloy_rpc_types_engine::ExecutionPayloadV2 {
    let withdrawals = to_alloy_withdrawals(payload.withdrawals);
    alloy_rpc_types_engine::ExecutionPayloadV2 {
        payload_inner: to_alloy_payload_v1!(payload),
        withdrawals,
    }
}

fn to_alloy_payload_v3(payload: ExecutionPayloadV3) -> alloy_rpc_types_engine::ExecutionPayloadV3 {
    let blob_gas_used = payload.blob_gas_used;
    let excess_blob_gas = payload.excess_blob_gas;
    let withdrawals = to_alloy_withdrawals(payload.withdrawals);
    alloy_rpc_types_engine::ExecutionPayloadV3 {
        payload_inner: alloy_rpc_types_engine::ExecutionPayloadV2 {
            payload_inner: to_alloy_payload_v1!(payload),
            withdrawals,
        },
        blob_gas_used,
        excess_blob_gas,
    }
}

fn to_alloy_payload_v4(payload: ExecutionPayloadV4) -> alloy_rpc_types_engine::ExecutionPayloadV4 {
    let blob_gas_used = payload.blob_gas_used;
    let excess_blob_gas = payload.excess_blob_gas;
    let block_access_list = Bytes::from(payload.block_access_list.into_inner());
    let slot_number = payload.slot_number;
    let withdrawals = to_alloy_withdrawals(payload.withdrawals);
    alloy_rpc_types_engine::ExecutionPayloadV4 {
        payload_inner: alloy_rpc_types_engine::ExecutionPayloadV3 {
            payload_inner: alloy_rpc_types_engine::ExecutionPayloadV2 {
                payload_inner: to_alloy_payload_v1!(payload),
                withdrawals,
            },
            blob_gas_used,
            excess_blob_gas,
        },
        block_access_list,
        slot_number,
    }
}

/// Converts canonical withdrawals into the alloy list. The list bound matches
/// the canonical bound.
fn to_alloy_withdrawals(withdrawals: Withdrawals) -> Vec<alloy_eips::eip4895::Withdrawal> {
    withdrawals
        .into_iter()
        .map(|withdrawal| alloy_eips::eip4895::Withdrawal {
            index: withdrawal.index,
            validator_index: withdrawal.validator_index,
            address: Address::from(withdrawal.address),
            amount: withdrawal.amount,
        })
        .collect()
}

fn cancun_fields(
    versioned_hashes: VersionedHashes,
    parent_beacon_block_root: Hash32,
) -> CancunPayloadFields {
    CancunPayloadFields::new(
        B256::from(parent_beacon_block_root),
        versioned_hashes.into_iter().map(B256::from).collect(),
    )
}

fn prague_fields(requests_hash: B256) -> alloy_rpc_types_engine::PraguePayloadFields {
    alloy_rpc_types_engine::PraguePayloadFields::new(requests_hash)
}

/// Computes the EIP-7685 requests hash over the Electra/Fulu execution requests.
fn compute_requests_hash_electra_fulu(
    requests: &ExecutionRequestsElectraFulu,
    hasher: &impl Sha256Hasher,
) -> B256 {
    let hashes = [
        encode_execution_requests(DEPOSIT_REQUEST_TYPE, &requests.deposits),
        encode_execution_requests(WITHDRAWAL_REQUEST_TYPE, &requests.withdrawals),
        encode_execution_requests(CONSOLIDATION_REQUEST_TYPE, &requests.consolidations),
    ]
    .into_iter()
    .flatten()
    .flat_map(|buf| hasher.hash(&buf))
    .collect::<Vec<_>>();
    hasher.hash(&hashes).into()
}

/// Computes the EIP-7685 requests hash over the Gloas execution requests.
fn compute_requests_hash_gloas(
    requests: &ExecutionRequestsGloas,
    hasher: &impl Sha256Hasher,
) -> B256 {
    let hashes = [
        encode_execution_requests(DEPOSIT_REQUEST_TYPE, &requests.deposits),
        encode_execution_requests(WITHDRAWAL_REQUEST_TYPE, &requests.withdrawals),
        encode_execution_requests(CONSOLIDATION_REQUEST_TYPE, &requests.consolidations),
        encode_execution_requests(BUILDER_DEPOSIT_REQUEST_TYPE, &requests.builder_deposits),
        encode_execution_requests(BUILDER_EXIT_REQUEST_TYPE, &requests.builder_exits),
    ]
    .into_iter()
    .flatten()
    .flat_map(|buf| hasher.hash(&buf))
    .collect::<Vec<_>>();
    hasher.hash(&hashes).into()
}

/// Encodes one request list as its type byte followed by the concatenated SSZ-encoded requests,
/// mirroring `encode_execution_requests` in [`requests.py`]. A list holding no items has no wire
/// form and contributes nothing to the commitment.
///
/// [`requests.py`]: https://github.com/ethereum/execution-specs/blob/tests-zkevm@v0.6.2/src/ethereum/forks/amsterdam/execution_engine/requests.py
fn encode_execution_requests<T: SszEncode>(request_type: u8, requests: &[T]) -> Option<Vec<u8>> {
    (!requests.is_empty()).then(|| {
        let mut buf = Vec::with_capacity(1 + requests.len() * T::fixed_size());
        buf.push(request_type);
        requests.iter().for_each(|request| request.ssz_append(&mut buf));
        buf
    })
}

/// Reconstructs the canonical payload request into a validated reth block.
fn to_reth_block(
    new_payload_request: NewPayloadRequest,
    chain_spec: Arc<ChainSpec>,
) -> Result<SealedBlock<Block<reth_ethereum_primitives::TransactionSigned>>, Error> {
    let execution_data = new_payload_request_to_execution_data(new_payload_request);
    ensure_well_formed_payload(chain_spec, execution_data)
}

/// Validates payload well-formedness, copied from [`validator.rs`] in the reth
/// ethereum payload builder. That crate pulls blst and other dependencies
/// unsuited to zkVM targets, so the function is vendored until it can be used
/// with minimal alloy-consensus features.
///
/// [`validator.rs`]: https://github.com/paradigmxyz/reth/blob/8eecad3d1d433ed509373713c21c31504290d17d/crates/ethereum/payload/src/validator.rs#L66
fn ensure_well_formed_payload<ChainSpec, T>(
    chain_spec: ChainSpec,
    payload: ExecutionData,
) -> Result<SealedBlock<Block<T>>, Error>
where
    ChainSpec: EthereumHardforks,
    T: SignedTransaction,
{
    let ExecutionData { payload, sidecar } = payload;

    let expected_hash = payload.block_hash();

    let sealed_block = payload.try_into_block_with_sidecar(&sidecar)?.seal_slow();

    // The hash included in the payload must match the computed block hash.
    if expected_hash != sealed_block.hash() {
        Err(PayloadError::BlockHash { execution: sealed_block.hash(), consensus: expected_hash })?;
    }

    shanghai::ensure_well_formed_fields(
        sealed_block.body(),
        chain_spec.is_shanghai_active_at_timestamp(sealed_block.timestamp),
    )?;

    cancun::ensure_well_formed_fields(
        &sealed_block,
        sidecar.cancun(),
        chain_spec.is_cancun_active_at_timestamp(sealed_block.timestamp),
    )?;

    prague::ensure_well_formed_fields(
        sealed_block.body(),
        sidecar.prague(),
        chain_spec.is_prague_active_at_timestamp(sealed_block.timestamp),
    )?;

    // TODO(Amsterdam) Add the Amsterdam specific validation.

    Ok(sealed_block)
}

/// Converts the canonical execution witness into the reth container.
fn to_reth_witness(witness: ExecutionWitness) -> stateless::ExecutionWitness {
    stateless::ExecutionWitness {
        state: to_bytes_vec(witness.state),
        codes: to_bytes_vec(witness.codes),
        keys: Vec::new(),
        headers: to_bytes_vec(witness.headers),
    }
}

/// Converts a canonical SSZ byte list collection into alloy byte vectors.
fn to_bytes_vec<const M: usize, const N: usize>(items: SszList<SszList<u8, M>, N>) -> Vec<Bytes> {
    items.into_iter().map(|item| Bytes::from(item.into_inner())).collect()
}
