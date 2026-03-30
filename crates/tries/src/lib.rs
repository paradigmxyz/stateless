//! Trie abstractions and implementations for stateless validation.

#![doc(
    html_logo_url = "https://raw.githubusercontent.com/paradigmxyz/reth/main/assets/reth-docs.png",
    html_favicon_url = "https://avatars0.githubusercontent.com/u/97369466?s=256",
    issue_tracker_base_url = "https://github.com/paradigmxyz/stateless/issues/"
)]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![cfg_attr(not(test), warn(unused_crate_dependencies))]
#![no_std]

extern crate alloc;

/// Default trie implementation based on `reth_trie_sparse`.
pub mod default;
mod error;
/// Zeth trie implementation backed by `zeth-mpt`.
pub mod zeth;

pub use error::{StatelessTrieError, WitnessDbError};

use alloy_primitives::{Address, B256, U256, map::B256Map};
use alloy_rpc_types_debug::ExecutionWitness;
use alloy_trie::TrieAccount;
use reth_trie_common::HashedPostState;
use revm_bytecode::Bytecode;

/// Trait for trie implementations that can be used for stateless validation.
pub trait StatelessTrie: core::fmt::Debug {
    /// Initialize the trie using the [`ExecutionWitness`].
    fn new(
        witness: &ExecutionWitness,
        pre_state_root: B256,
    ) -> Result<(Self, B256Map<Bytecode>), StatelessTrieError>
    where
        Self: Sized;

    /// Returns the [`TrieAccount`] that corresponds to the [`Address`].
    fn account(&self, address: Address) -> Result<Option<TrieAccount>, WitnessDbError>;

    /// Returns the storage slot value that corresponds to the `(address, slot)` tuple.
    fn storage(&self, address: Address, slot: U256) -> Result<U256, WitnessDbError>;

    /// Computes the new state root from the [`HashedPostState`].
    fn calculate_state_root(&mut self, state: HashedPostState) -> Result<B256, StatelessTrieError>;
}
