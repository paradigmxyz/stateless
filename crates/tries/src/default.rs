use crate::{StatelessTrie, StatelessTrieError, WitnessDbError};
use alloc::{collections::VecDeque, format, vec::Vec};
use alloy_primitives::{Address, B256, U256, keccak256, map::B256IndexMap};
use alloy_rlp::Decodable;
use alloy_rpc_types_debug::ExecutionWitness;
use alloy_trie::{EMPTY_ROOT_HASH, TrieAccount, nodes::TrieNode};
use itertools::Itertools;
use reth_trie_common::{
    BranchNodeMasks, DecodedMultiProofV2, HashedPostState, Nibbles, ProofTrieNodeV2,
};
use reth_trie_sparse::{
    RevealableSparseTrie, SparseStateTrie,
    errors::SparseStateTrieResult,
    provider::{DefaultTrieNodeProvider, DefaultTrieNodeProviderFactory},
};
use revm_bytecode::Bytecode;

/// `StatelessSparseTrie` structure for usage during stateless validation
#[derive(Debug)]
pub struct StatelessSparseTrie {
    inner: SparseStateTrie,
}

impl StatelessSparseTrie {
    /// Initialize the stateless trie using the `ExecutionWitness`
    ///
    /// Note: Currently this method does not check that the `ExecutionWitness`
    /// is complete for all of the preimage keys.
    pub fn new(
        witness: &ExecutionWitness,
        pre_state_root: B256,
    ) -> Result<(Self, B256IndexMap<Bytecode>), StatelessTrieError> {
        verify_execution_witness(witness, pre_state_root)
            .map(|(inner, bytecode)| (Self { inner }, bytecode))
    }

    /// Returns the `TrieAccount` that corresponds to the `Address`
    ///
    /// This method will error if the `ExecutionWitness` is not able to guarantee
    /// that the account is missing from the Trie _and_ the witness was complete.
    pub fn account(&self, address: Address) -> Result<Option<TrieAccount>, WitnessDbError> {
        let hashed_address = keccak256(address);

        if let Some(bytes) = self.inner.get_account_value(&hashed_address) {
            let account = TrieAccount::decode(&mut bytes.as_slice())?;
            return Ok(Some(account));
        }

        if !self.inner.is_account_revealed(hashed_address) {
            return Err(WitnessDbError::TrieWitness(format!(
                "incomplete account witness for {hashed_address:?}"
            )));
        }

        Ok(None)
    }

    /// Returns the storage slot value that corresponds to the given (address, slot) tuple.
    ///
    /// This method will error if the `ExecutionWitness` is not able to guarantee
    /// that the storage was missing from the Trie _and_ the witness was complete.
    pub fn storage(&self, address: Address, slot: U256) -> Result<U256, WitnessDbError> {
        let hashed_address = keccak256(address);
        let hashed_slot = keccak256(B256::from(slot));

        if let Some(raw) = self.inner.get_storage_slot_value(&hashed_address, &hashed_slot) {
            return Ok(U256::decode(&mut raw.as_slice())?);
        }

        // Storage slot value is not present in the trie, validate that the witness is complete.
        // If the account exists in the trie...
        if let Some(bytes) = self.inner.get_account_value(&hashed_address) {
            // ...check that its storage is either empty or the storage trie was sufficiently
            // revealed...
            let account = TrieAccount::decode(&mut bytes.as_slice())?;
            if account.storage_root != EMPTY_ROOT_HASH
                && !self.inner.check_valid_storage_witness(hashed_address, hashed_slot)
            {
                return Err(WitnessDbError::TrieWitness(format!(
                    "incomplete storage witness: prover must supply exclusion proof for slot {hashed_slot:?} in account {hashed_address:?}"
                )));
            }
        } else if !self.inner.is_account_revealed(hashed_address) {
            // ...else if account is missing, validate that the account trie was sufficiently
            // revealed.
            return Err(WitnessDbError::TrieWitness(format!(
                "incomplete account witness for {hashed_address:?}"
            )));
        }

        Ok(U256::ZERO)
    }

    /// Computes the new state root from the `HashedPostState`.
    pub fn calculate_state_root(
        &mut self,
        state: HashedPostState,
    ) -> Result<B256, StatelessTrieError> {
        calculate_state_root(&mut self.inner, state)
            .map_err(|_e| StatelessTrieError::StatelessStateRootCalculationFailed)
    }
}

impl StatelessTrie for StatelessSparseTrie {
    fn new(
        witness: &ExecutionWitness,
        pre_state_root: B256,
    ) -> Result<(Self, B256IndexMap<Bytecode>), StatelessTrieError> {
        Self::new(witness, pre_state_root)
    }

    fn account(&self, address: Address) -> Result<Option<TrieAccount>, WitnessDbError> {
        self.account(address)
    }

    fn storage(&self, address: Address, slot: U256) -> Result<U256, WitnessDbError> {
        self.storage(address, slot)
    }

    fn calculate_state_root(&mut self, state: HashedPostState) -> Result<B256, StatelessTrieError> {
        self.calculate_state_root(state)
    }
}

/// Verifies execution witness [`ExecutionWitness`] against an expected pre-state root.
///
/// This function takes the RLP-encoded values provided in [`ExecutionWitness`]
/// (which includes state trie nodes, storage trie nodes, and contract bytecode)
/// and uses it to populate a new [`SparseStateTrie`].
///
/// If the computed root hash matches the `pre_state_root`, it signifies that the
/// provided execution witness is consistent with that pre-state root. In this case, the function
/// returns the populated [`SparseStateTrie`] and a [`B256IndexMap`] containing the
/// contract bytecode (mapping code hash to [`Bytecode`]).
///
/// The bytecode has a separate mapping because the [`SparseStateTrie`] does not store the
/// contract bytecode, only the hash of it (code hash).
///
/// If the roots do not match, it returns an error indicating the witness is invalid
/// for the given `pre_state_root` (see [`StatelessTrieError::PreStateRootMismatch`]).
fn verify_execution_witness(
    witness: &ExecutionWitness,
    pre_state_root: B256,
) -> Result<(SparseStateTrie, B256IndexMap<Bytecode>), StatelessTrieError> {
    let provider_factory = DefaultTrieNodeProviderFactory;
    let mut trie = SparseStateTrie::new();
    let mut bytecode = B256IndexMap::default();

    // Build a hash-indexed map of witness nodes.
    let mut nodes_by_hash = B256IndexMap::default();
    for rlp_encoded in &witness.state {
        let hash = keccak256(rlp_encoded);
        nodes_by_hash.insert(hash, rlp_encoded.clone());
    }
    for rlp_encoded in &witness.codes {
        let hash = keccak256(rlp_encoded);
        bytecode.insert(hash, Bytecode::new_raw(rlp_encoded.clone()));
    }

    // Build a DecodedMultiProofV2 by walking the witness from the root.
    let multiproof = build_multiproof_from_witness(pre_state_root, &nodes_by_hash)
        .map_err(|_| StatelessTrieError::WitnessRevealFailed { pre_state_root })?;

    // Reveal the witness into the sparse trie.
    trie.reveal_decoded_multiproof_v2(multiproof)
        .map_err(|_e| StatelessTrieError::WitnessRevealFailed { pre_state_root })?;

    // Calculate the root
    let computed_root = trie
        .root(&provider_factory)
        .map_err(|_e| StatelessTrieError::StatelessPreStateRootCalculationFailed)?;

    if computed_root == pre_state_root {
        Ok((trie, bytecode))
    } else {
        Err(StatelessTrieError::PreStateRootMismatch {
            got: computed_root,
            expected: pre_state_root,
        })
    }
}

/// Builds a [`DecodedMultiProofV2`] from a flat witness map by walking the trie from the root.
///
/// This replicates the old `SparseStateTrie::reveal_witness` logic, but outputs the V2 multiproof
/// structure needed by the current API.
fn build_multiproof_from_witness(
    state_root: B256,
    nodes_by_hash: &B256IndexMap<alloy_primitives::Bytes>,
) -> Result<DecodedMultiProofV2, alloy_rlp::Error> {
    #[allow(clippy::disallowed_types)]
    use alloy_primitives::map::B256Map;

    let mut account_nodes: Vec<(Nibbles, TrieNode, Option<BranchNodeMasks>)> = Vec::new();
    #[allow(clippy::disallowed_types)]
    let mut storage_nodes: B256Map<Vec<(Nibbles, TrieNode, Option<BranchNodeMasks>)>> =
        B256Map::default();

    // BFS queue: (hash, path, maybe_account_address)
    // When maybe_account is None, we're traversing the account trie.
    // When Some(hashed_address), we're traversing that account's storage trie.
    let mut queue: VecDeque<(B256, Nibbles, Option<B256>)> =
        VecDeque::from([(state_root, Nibbles::default(), None)]);

    while let Some((hash, path, maybe_account)) = queue.pop_front() {
        let Some(trie_node_bytes) = nodes_by_hash.get(&hash) else { continue };
        let trie_node = TrieNode::decode(&mut &trie_node_bytes[..])?;

        // Push children nodes into the queue.
        match &trie_node {
            TrieNode::Branch(branch) => {
                for (idx, maybe_child) in branch.as_ref().children() {
                    if let Some(child_hash) =
                        maybe_child.and_then(alloy_trie::nodes::RlpNode::as_hash)
                    {
                        let mut child_path = path;
                        child_path.push_unchecked(idx);
                        queue.push_back((child_hash, child_path, maybe_account));
                    }
                }
            }
            TrieNode::Extension(ext) => {
                if let Some(child_hash) = ext.child.as_hash() {
                    let mut child_path = path;
                    child_path.extend(&ext.key);
                    queue.push_back((child_hash, child_path, maybe_account));
                }
            }
            TrieNode::Leaf(leaf) => {
                if maybe_account.is_none() {
                    // Account trie leaf: decode the account and enqueue storage trie if needed.
                    let mut full_path = path;
                    full_path.extend(&leaf.key);
                    let hashed_address = B256::from_slice(&full_path.pack());
                    let account = TrieAccount::decode(&mut &leaf.value[..])?;
                    if account.storage_root != EMPTY_ROOT_HASH {
                        queue.push_back((
                            account.storage_root,
                            Nibbles::default(),
                            Some(hashed_address),
                        ));
                    }
                }
            }
            TrieNode::EmptyRoot => {}
        }

        // Record the node for the appropriate trie.
        if let Some(account) = maybe_account {
            storage_nodes.entry(account).or_default().push((path, trie_node, None));
        } else {
            account_nodes.push((path, trie_node, None));
        }
    }

    // Sort nodes in depth-first order (children before parents) as required by
    // ProofTrieNodeV2::from_sorted_trie_nodes.
    account_nodes.sort_by(|(a, _, _), (b, _, _)| b.cmp(a));
    let account_proofs = ProofTrieNodeV2::from_sorted_trie_nodes(account_nodes);

    #[allow(clippy::disallowed_types)]
    let mut storage_proofs = B256Map::default();
    for (account, mut nodes) in storage_nodes {
        nodes.sort_by(|(a, _, _), (b, _, _)| b.cmp(a));
        storage_proofs.insert(account, ProofTrieNodeV2::from_sorted_trie_nodes(nodes));
    }

    Ok(DecodedMultiProofV2 { account_proofs, storage_proofs })
}

// Copied and modified from ress: https://github.com/paradigmxyz/ress/blob/06bf2c4788e45b8fcbd640e38b6243e6f87c4d0e/crates/engine/src/tree/root.rs
/// Calculates the post-execution state root by applying state changes to a sparse trie.
///
/// This function takes a [`SparseStateTrie`] with the pre-state and a [`HashedPostState`]
/// containing account and storage changes resulting from block execution (state diff).
///
/// It modifies the input `trie` in place to reflect these changes and then calculates the
/// final post-execution state root.
fn calculate_state_root(
    trie: &mut SparseStateTrie,
    state: HashedPostState,
) -> SparseStateTrieResult<B256> {
    // 1. Apply storage‑slot updates and compute each contract's storage root
    //
    //
    // We walk over every (address, storage) pair in deterministic order
    // and update the corresponding per‑account storage trie in‑place.
    // When we're done we collect (address, updated_storage_trie) in a `Vec`
    // so that we can insert them back into the outer state trie afterwards ― this avoids
    // borrowing issues.
    let mut storage_results = Vec::with_capacity(state.storages.len());

    // In `verify_execution_witness` a `DefaultTrieNodeProviderFactory` is used, so we use the same
    // again in here.
    let provider_factory = DefaultTrieNodeProviderFactory;
    let storage_provider = DefaultTrieNodeProvider;

    for (address, storage) in state.storages.into_iter().sorted_unstable_by_key(|(addr, _)| *addr) {
        // Take the existing storage trie (or create an empty, "revealed" one)
        let mut storage_trie =
            trie.take_storage_trie(&address).unwrap_or_else(RevealableSparseTrie::revealed_empty);

        if storage.wiped {
            storage_trie.wipe()?;
        }

        // Apply slot‑level changes
        for (hashed_slot, value) in
            storage.storage.into_iter().sorted_unstable_by_key(|(slot, _)| *slot)
        {
            let nibbles = Nibbles::unpack(hashed_slot);
            if value.is_zero() {
                storage_trie.remove_leaf(&nibbles, &storage_provider)?;
            } else {
                storage_trie.update_leaf(
                    nibbles,
                    alloy_rlp::encode_fixed_size(&value).to_vec(),
                    &storage_provider,
                )?;
            }
        }

        // Finalise the storage‑trie root before pushing the result
        storage_trie.root();
        storage_results.push((address, storage_trie));
    }

    // Insert every updated storage trie back into the outer state trie
    for (address, storage_trie) in storage_results {
        trie.insert_storage_trie(address, storage_trie);
    }

    // 2. Apply account‑level updates and (re)encode the account nodes
    for (hashed_address, account) in
        state.accounts.into_iter().sorted_unstable_by_key(|(addr, _)| *addr)
    {
        trie.update_account_stateless(hashed_address, account, &provider_factory)?;
    }

    // Return new state root
    trie.root(&provider_factory)
}
