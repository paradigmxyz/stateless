use crate::{
    ExecutionWitness,
    recover_block::{UncompressedPublicKey, recover_block_with_public_keys},
    witness_db::WitnessDatabase,
};
use alloc::{
    collections::BTreeMap,
    fmt::Debug,
    string::{String, ToString},
    sync::Arc,
    vec::Vec,
};
use alloy_consensus::{BlockHeader, Header};
use alloy_primitives::{B256, keccak256};
use reth_chainspec::{EthChainSpec, EthereumHardforks};
use reth_consensus::ConsensusError;
use reth_consensus::{Consensus, HeaderValidator};
use reth_ethereum_consensus::{EthBeaconConsensus, validate_block_post_execution};
use reth_ethereum_primitives::{Block, EthPrimitives, EthereumReceipt};
use reth_evm::{
    ConfigureEvm,
    execute::{BlockExecutionOutput, Executor},
};
use reth_primitives_traits::{RecoveredBlock, SealedHeader};
use reth_trie_common::{HashedPostState, KeccakKeyHasher};
use tries::{StatelessTrie, StatelessTrieError, default::StatelessSparseTrie};

/// BLOCKHASH ancestor lookup window limit per EVM (number of most recent blocks accessible).
const BLOCKHASH_ANCESTOR_LIMIT: usize = 256;

/// Errors that can occur during stateless validation.
#[derive(Debug, thiserror::Error)]
pub enum StatelessValidationError {
    /// Error when the number of ancestor headers exceeds the limit.
    #[error("ancestor header count ({count}) exceeds limit ({limit})")]
    AncestorHeaderLimitExceeded {
        /// The number of headers provided.
        count: usize,
        /// The limit.
        limit: usize,
    },

    /// Error when the ancestor headers do not form a contiguous chain.
    #[error("invalid ancestor chain")]
    InvalidAncestorChain,

    /// Error when revealing the witness data failed.
    #[error("failed to reveal witness data for pre-state root {pre_state_root}")]
    WitnessRevealFailed {
        /// The pre-state root used for verification.
        pre_state_root: B256,
    },

    /// Error during stateless block execution.
    #[error("stateless block execution failed: {0}")]
    StatelessExecutionFailed(String),

    /// Error during consensus validation of the block.
    #[error("consensus validation failed: {0}")]
    ConsensusValidationFailed(#[from] ConsensusError),

    /// Error during stateless state root calculation.
    #[error("stateless state root calculation failed")]
    StatelessStateRootCalculationFailed,

    /// Error calculating the pre-state root from the witness data.
    #[error("stateless pre-state root calculation failed")]
    StatelessPreStateRootCalculationFailed,

    /// Error when required ancestor headers are missing (e.g., parent header for pre-state root).
    #[error("missing required ancestor headers")]
    MissingAncestorHeader,

    /// Error when deserializing ancestor headers
    #[error("could not deserialize ancestor headers")]
    HeaderDeserializationFailed,

    /// Error when the computed state root does not match the one in the block header.
    #[error("mismatched post-state root: {got}\n {expected}")]
    PostStateRootMismatch {
        /// The computed post-state root
        got: B256,
        /// The expected post-state root; in the block header
        expected: B256,
    },

    /// Error when the computed pre-state root does not match the expected one.
    #[error("mismatched pre-state root: {got} \n {expected}")]
    PreStateRootMismatch {
        /// The computed pre-state root
        got: B256,
        /// The expected pre-state root from the previous block
        expected: B256,
    },

    /// Error during signer recovery.
    #[error("signer recovery failed")]
    SignerRecovery,

    /// Error when signature has non-normalized s value in homestead block.
    #[error("signature s value not normalized for homestead block")]
    HomesteadSignatureNotNormalized,

    /// Custom error.
    #[error("{0}")]
    Custom(&'static str),
}

impl From<StatelessTrieError> for StatelessValidationError {
    fn from(err: StatelessTrieError) -> Self {
        match err {
            StatelessTrieError::WitnessRevealFailed { pre_state_root } => {
                Self::WitnessRevealFailed { pre_state_root }
            }
            StatelessTrieError::StatelessStateRootCalculationFailed => {
                Self::StatelessStateRootCalculationFailed
            }
            StatelessTrieError::StatelessPreStateRootCalculationFailed => {
                Self::StatelessPreStateRootCalculationFailed
            }
            StatelessTrieError::PreStateRootMismatch { got, expected } => {
                Self::PreStateRootMismatch { got, expected }
            }
        }
    }
}

/// Performs stateless validation of a block using the provided witness data.
pub fn stateless_validation<ChainSpec, E>(
    current_block: Block,
    public_keys: Vec<UncompressedPublicKey>,
    witness: ExecutionWitness,
    chain_spec: Arc<ChainSpec>,
    evm_config: E,
) -> Result<(B256, BlockExecutionOutput<EthereumReceipt>), StatelessValidationError>
where
    ChainSpec: Send + Sync + EthChainSpec<Header = Header> + EthereumHardforks + Debug,
    E: ConfigureEvm<Primitives = EthPrimitives> + Clone + 'static,
{
    stateless_validation_with_trie::<StatelessSparseTrie, ChainSpec, E>(
        current_block,
        public_keys,
        witness,
        chain_spec,
        evm_config,
    )
}

/// Performs stateless validation of a block using a custom `StatelessTrie` implementation.
pub fn stateless_validation_with_trie<T, ChainSpec, E>(
    current_block: Block,
    public_keys: Vec<UncompressedPublicKey>,
    witness: ExecutionWitness,
    chain_spec: Arc<ChainSpec>,
    evm_config: E,
) -> Result<(B256, BlockExecutionOutput<EthereumReceipt>), StatelessValidationError>
where
    T: StatelessTrie,
    ChainSpec: Send + Sync + EthChainSpec<Header = Header> + EthereumHardforks + Debug,
    E: ConfigureEvm<Primitives = EthPrimitives> + Clone + 'static,
{
    let current_block = recover_block_with_public_keys(current_block, public_keys, &*chain_spec)?;

    let mut ancestor_headers: Vec<_> = witness
        .headers
        .iter()
        .map(|bytes| {
            let hash = keccak256(bytes);
            alloy_rlp::decode_exact::<Header>(bytes)
                .map(|h| SealedHeader::new(h, hash))
                .map_err(|_| StatelessValidationError::HeaderDeserializationFailed)
        })
        .collect::<Result<_, _>>()?;
    ancestor_headers.sort_by_key(|header| header.number());

    let count = ancestor_headers.len();
    if count > BLOCKHASH_ANCESTOR_LIMIT {
        return Err(StatelessValidationError::AncestorHeaderLimitExceeded {
            count,
            limit: BLOCKHASH_ANCESTOR_LIMIT,
        });
    }

    let ancestor_hashes = compute_ancestor_hashes(&current_block, &ancestor_headers)?;

    let parent = match ancestor_headers.last() {
        Some(prev_header) => prev_header,
        None => return Err(StatelessValidationError::MissingAncestorHeader),
    };

    validate_block_consensus(chain_spec.clone(), &current_block, parent)?;

    let (mut trie, bytecode) = T::new(&witness, parent.state_root)?;

    let db = WitnessDatabase::new(&trie, bytecode, ancestor_hashes);

    let executor = evm_config.executor(db);
    let output = executor
        .execute(&current_block)
        .map_err(|e| StatelessValidationError::StatelessExecutionFailed(e.to_string()))?;

    validate_block_post_execution(
        &current_block,
        &chain_spec,
        &output.receipts,
        &output.requests,
        None,
    )
    .map_err(StatelessValidationError::ConsensusValidationFailed)?;

    let hashed_state = HashedPostState::from_bundle_state::<KeccakKeyHasher>(&output.state.state);
    let state_root = trie.calculate_state_root(hashed_state)?;
    if state_root != current_block.state_root {
        return Err(StatelessValidationError::PostStateRootMismatch {
            got: state_root,
            expected: current_block.state_root,
        });
    }

    Ok((current_block.hash_slow(), output))
}

fn validate_block_consensus<ChainSpec>(
    chain_spec: Arc<ChainSpec>,
    block: &RecoveredBlock<Block>,
    parent: &SealedHeader<Header>,
) -> Result<(), StatelessValidationError>
where
    ChainSpec: Send + Sync + EthChainSpec<Header = Header> + EthereumHardforks + Debug,
{
    let consensus = EthBeaconConsensus::new(chain_spec);

    consensus.validate_header(block.sealed_header())?;
    consensus.validate_header_against_parent(block.sealed_header(), parent)?;

    consensus.validate_block_pre_execution(block)?;

    Ok(())
}

fn compute_ancestor_hashes(
    current_block: &RecoveredBlock<Block>,
    ancestor_headers: &[SealedHeader],
) -> Result<BTreeMap<u64, B256>, StatelessValidationError> {
    let mut ancestor_hashes = BTreeMap::new();

    let mut child_header = current_block.sealed_header();

    for parent_header in ancestor_headers.iter().rev() {
        let parent_hash = child_header.parent_hash();
        ancestor_hashes.insert(parent_header.number, parent_hash);

        if parent_hash != parent_header.hash() {
            return Err(StatelessValidationError::InvalidAncestorChain);
        }

        if parent_header.number + 1 != child_header.number {
            return Err(StatelessValidationError::InvalidAncestorChain);
        }

        child_header = parent_header
    }

    Ok(ancestor_hashes)
}
