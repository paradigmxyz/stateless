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
#[cfg(feature = "std")]
pub mod default;
mod error;
/// Zeth trie implementation backed by `zeth-mpt`.
pub mod zeth;

pub use error::{StatelessTrieError, WitnessDbError};

use alloy_primitives::{Address, B256, Bytes, U256, map::B256IndexMap};
use alloy_rpc_types_debug::ExecutionWitness;
use alloy_trie::TrieAccount;
use reth_trie_common::HashedPostState;

/// Trait for trie implementations that can be used for stateless validation.
pub trait StatelessTrie: core::fmt::Debug {
    /// Initialize the trie using the [`ExecutionWitness`].
    fn new(
        witness: ExecutionWitness,
        pre_state_root: B256,
    ) -> Result<(Self, B256IndexMap<Bytes>), StatelessTrieError>
    where
        Self: Sized;

    /// Returns the [`TrieAccount`] that corresponds to the [`Address`].
    fn account(&self, address: Address) -> Result<Option<TrieAccount>, WitnessDbError>;

    /// Returns the storage slot value that corresponds to the `(address, slot)` tuple.
    fn storage(&self, address: Address, slot: U256) -> Result<U256, WitnessDbError>;

    /// Clears an account's storage before applying its post-state changes.
    ///
    /// The address is hashed, as in [`HashedPostState`]. Clearing must also discard blinded slots.
    fn clear_storage(&mut self, hashed_address: B256);

    /// Computes the new state root from the [`HashedPostState`].
    fn calculate_state_root(&mut self, state: HashedPostState) -> Result<B256, StatelessTrieError>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy_primitives::{U256, keccak256};
    use reth_trie_common::HashedStorage;
    use zeth_mpt::Trie;

    fn storage_reset_discards_unrevealed_slots<T: StatelessTrie>(new_value: U256) {
        let address = Address::with_last_byte(1);
        let hashed_address = keccak256(address);
        let old_slot = U256::from(1);
        let new_slot = U256::from(2);

        let mut storage = Trie::default();
        storage.insert(keccak256(B256::from(old_slot)), alloy_rlp::encode(U256::from(42)));
        let mut account = TrieAccount { storage_root: storage.hash_slow(), ..Default::default() };
        let mut state = Trie::default();
        state.insert(hashed_address, alloy_rlp::encode(account));

        // A storage reset must not require witnesses for the old slots.
        let witness = ExecutionWitness { state: state.rlp_nodes(), ..Default::default() };
        let (mut trie, _) = T::new(witness, state.hash_slow()).unwrap();
        trie.clear_storage(hashed_address);

        let mut post_state = HashedPostState::default();
        post_state.accounts.insert(hashed_address, Some(Default::default()));
        post_state.storages.insert(
            hashed_address,
            if new_value.is_zero() {
                HashedStorage::default()
            } else {
                HashedStorage::from_iter([(keccak256(B256::from(new_slot)), new_value)])
            },
        );

        storage.clear();
        if !new_value.is_zero() {
            storage.insert(keccak256(B256::from(new_slot)), alloy_rlp::encode(new_value));
        }
        account.storage_root = storage.hash_slow();
        state.insert(hashed_address, alloy_rlp::encode(account));
        assert_eq!(trie.calculate_state_root(post_state).unwrap(), state.hash_slow());
        assert_eq!(trie.storage(address, old_slot).unwrap(), U256::ZERO);
        assert_eq!(trie.storage(address, new_slot).unwrap(), new_value);
    }

    #[test]
    #[cfg(feature = "std")]
    fn reth_storage_reset_discards_unrevealed_slots() {
        for new_value in [U256::ZERO, U256::from(7)] {
            storage_reset_discards_unrevealed_slots::<crate::default::StatelessSparseTrie>(
                new_value,
            );
        }
    }

    #[test]
    fn zeth_storage_reset_discards_unrevealed_slots() {
        for new_value in [U256::ZERO, U256::from(7)] {
            storage_reset_discards_unrevealed_slots::<crate::zeth::SparseState>(new_value);
        }
    }
}
