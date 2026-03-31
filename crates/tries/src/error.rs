use alloc::string::String;
use alloy_primitives::B256;

/// Errors originating from trie construction and root computation.
#[derive(Debug, thiserror::Error)]
pub enum StatelessTrieError {
    /// Error when revealing the witness data failed.
    #[error("failed to reveal witness data for pre-state root {pre_state_root}")]
    WitnessRevealFailed {
        /// The pre-state root used for verification.
        pre_state_root: B256,
    },

    /// Error during state root calculation.
    #[error("stateless state root calculation failed")]
    StatelessStateRootCalculationFailed,

    /// Error calculating the pre-state root from the witness data.
    #[error("stateless pre-state root calculation failed")]
    StatelessPreStateRootCalculationFailed,

    /// Error when the computed pre-state root does not match the expected one.
    #[error("mismatched pre-state root: {got} \n {expected}")]
    PreStateRootMismatch {
        /// The computed pre-state root.
        got: B256,
        /// The expected pre-state root.
        expected: B256,
    },
}

/// Error type for witness-backed database operations.
#[derive(Debug, thiserror::Error)]
pub enum WitnessDbError {
    /// Incomplete or missing witness data.
    #[error("trie witness error: {0}")]
    TrieWitness(String),
    /// Missing state for a block number.
    #[error("state for block {0} not found")]
    StateNotFound(u64),
    /// RLP decoding error.
    #[error("RLP decode error: {0}")]
    Rlp(#[from] alloy_rlp::Error),
}

impl revm_database_interface::DBErrorMarker for WitnessDbError {}
