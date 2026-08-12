//! Provides the [`WitnessDatabase`] type, an implementation of [`revm_database_interface::Database`]
//! specifically designed for stateless execution environments.

use alloc::{collections::btree_map::BTreeMap, format};
use alloy_primitives::{Address, B256, Bytes, U256, map::B256IndexMap};
use alloy_trie::EMPTY_ROOT_HASH;
use revm_bytecode::Bytecode;
use revm_database_interface::Database;
use revm_state::AccountInfo;
use tries::{StatelessTrie, WitnessDbError};

/// An EVM database implementation backed by witness data.
///
/// This struct implements the [`revm_database_interface::Database`] trait, allowing the EVM to execute
/// transactions using:
///  - Account and storage slot data provided by a [`StatelessTrie`] implementation.
///  - Bytecode and ancestor block hashes provided by in-memory maps.
///
/// This is designed for stateless execution scenarios where direct access to a full node's
/// database is not available or desired.
#[derive(Debug)]
pub(crate) struct WitnessDatabase<'a, T>
where
    T: StatelessTrie,
{
    /// Map of block numbers to block hashes.
    /// This is used to service the `BLOCKHASH` opcode.
    block_hashes_by_block_number: BTreeMap<u64, B256>,
    /// Map of code hashes to bytecode.
    /// Used to fetch contract code needed during execution.
    bytecode: B256IndexMap<Bytes>,
    /// The sparse Merkle Patricia Trie containing account and storage state.
    /// This is used to provide account/storage values during EVM execution.
    trie: &'a T,
    /// Whether accounts loaded from the trie have non-empty storage.
    storage_presence: BTreeMap<Address, bool>,
}

impl<'a, T> WitnessDatabase<'a, T>
where
    T: StatelessTrie,
{
    /// Creates a new [`WitnessDatabase`] instance.
    ///
    /// # Assumptions
    ///
    /// This function assumes:
    /// 1. The provided `trie` has been populated with state data consistent with a known state root
    ///    (e.g., using witness data and verifying against a parent block's state root).
    /// 2. The `bytecode` map contains all bytecode corresponding to code hashes present in the
    ///    account data within the `trie`.
    /// 3. The `ancestor_hashes` map contains the block hashes for the relevant ancestor blocks (up
    ///    to 256 including the current block number). It assumes these hashes correspond to a
    ///    contiguous chain of blocks. The caller is responsible for verifying the contiguity and
    ///    the block limit.
    pub(crate) const fn new(
        trie: &'a T,
        bytecode: B256IndexMap<Bytes>,
        ancestor_hashes: BTreeMap<u64, B256>,
    ) -> Self {
        Self {
            trie,
            block_hashes_by_block_number: ancestor_hashes,
            bytecode,
            storage_presence: BTreeMap::new(),
        }
    }
}

impl<T> Database for WitnessDatabase<'_, T>
where
    T: StatelessTrie,
{
    /// The database error type.
    type Error = WitnessDbError;

    /// Get basic account information by hashing the address and looking up the account RLP
    /// in the underlying [`StatelessTrie`] implementation.
    ///
    /// Returns `Ok(None)` if the account is not found in the trie.
    fn basic(&mut self, address: Address) -> Result<Option<AccountInfo>, Self::Error> {
        self.trie.account(address).map(|account| {
            self.storage_presence.insert(
                address,
                account.as_ref().is_some_and(|account| account.storage_root != EMPTY_ROOT_HASH),
            );
            account.map(|account| AccountInfo {
                balance: account.balance,
                nonce: account.nonce,
                code_hash: account.code_hash,
                code: None,
                account_id: None,
            })
        })
    }

    /// Returns whether the account has any non-zero storage slots.
    fn account_has_storage(&mut self, address: Address) -> Result<bool, Self::Error> {
        if let Some(has_storage) = self.storage_presence.get(&address) {
            return Ok(*has_storage);
        }

        let has_storage = self
            .trie
            .account(address)?
            .is_some_and(|account| account.storage_root != EMPTY_ROOT_HASH);
        self.storage_presence.insert(address, has_storage);
        Ok(has_storage)
    }

    /// Get storage value of an account at a specific slot.
    ///
    /// Returns `U256::ZERO` if the slot is not found in the trie.
    fn storage(&mut self, address: Address, slot: U256) -> Result<U256, Self::Error> {
        self.trie.storage(address, slot)
    }

    /// Get account code by its hash from the provided bytecode map.
    ///
    /// Returns an error if the bytecode for the given hash is not found in the map.
    fn code_by_hash(&mut self, code_hash: B256) -> Result<Bytecode, Self::Error> {
        let raw = self.bytecode.get(&code_hash).cloned().ok_or_else(|| {
            WitnessDbError::TrieWitness(format!("bytecode for {code_hash} not found"))
        })?;
        Ok(Bytecode::new_raw(raw))
    }

    /// Get block hash by block number from the provided ancestor hashes map.
    ///
    /// Returns an error if the hash for the given block number is not found in the map.
    fn block_hash(&mut self, block_number: u64) -> Result<B256, Self::Error> {
        self.block_hashes_by_block_number
            .get(&block_number)
            .copied()
            .ok_or(WitnessDbError::StateNotFound(block_number))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy_primitives::KECCAK256_EMPTY;
    use alloy_rpc_types_debug::ExecutionWitness;
    use alloy_trie::TrieAccount;
    use core::cell::Cell;
    use reth_trie_common::HashedPostState;
    use tries::StatelessTrieError;

    #[derive(Debug)]
    struct TestTrie {
        account: Option<TrieAccount>,
        account_lookups: Cell<usize>,
    }

    impl StatelessTrie for TestTrie {
        fn new(
            _witness: ExecutionWitness,
            _pre_state_root: B256,
        ) -> Result<(Self, B256IndexMap<Bytes>), StatelessTrieError> {
            Err(StatelessTrieError::StatelessPreStateRootCalculationFailed)
        }

        fn account(&self, _address: Address) -> Result<Option<TrieAccount>, WitnessDbError> {
            self.account_lookups.set(self.account_lookups.get() + 1);
            Ok(self.account)
        }

        fn storage(&self, _address: Address, _slot: U256) -> Result<U256, WitnessDbError> {
            Ok(U256::ZERO)
        }

        fn calculate_state_root(
            &mut self,
            _state: HashedPostState,
        ) -> Result<B256, StatelessTrieError> {
            Ok(B256::ZERO)
        }
    }

    #[test]
    fn basic_caches_storage_presence() {
        let address = Address::with_last_byte(1);
        let trie = TestTrie {
            account: Some(TrieAccount {
                nonce: 0,
                balance: U256::ZERO,
                storage_root: B256::with_last_byte(1),
                code_hash: KECCAK256_EMPTY,
            }),
            account_lookups: Cell::new(0),
        };
        let mut db = WitnessDatabase::new(&trie, B256IndexMap::default(), BTreeMap::new());

        assert!(db.basic(address).unwrap().is_some());
        assert!(db.account_has_storage(address).unwrap());
        assert_eq!(trie.account_lookups.get(), 1);
    }

    #[test]
    fn storage_presence_lookup_is_cached() {
        let address = Address::with_last_byte(1);
        let trie = TestTrie { account: None, account_lookups: Cell::new(0) };
        let mut db = WitnessDatabase::new(&trie, B256IndexMap::default(), BTreeMap::new());

        assert!(!db.account_has_storage(address).unwrap());
        assert!(!db.account_has_storage(address).unwrap());
        assert_eq!(trie.account_lookups.get(), 1);
    }
}
