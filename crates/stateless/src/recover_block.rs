use crate::validation::StatelessValidationError;
use alloc::vec::Vec;
use alloy_consensus::BlockHeader;
use alloy_primitives::{Address, B256, Signature};
use core::ops::Deref;
use reth_chainspec::EthereumHardforks;
use reth_ethereum_primitives::{Block, TransactionSigned};
use reth_primitives_traits::{Block as _, RecoveredBlock};
use serde::{Deserialize, Serialize};
use serde_with::{Bytes, serde_as};

/// Serialized uncompressed public key
#[serde_as]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UncompressedPublicKey(#[serde_as(as = "Bytes")] pub [u8; 65]);

impl Deref for UncompressedPublicKey {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// Verifies all transactions in a block against a list of public keys and signatures.
///
/// Returns a `RecoveredBlock`
pub fn recover_block_with_public_keys<ChainSpec>(
    block: Block,
    public_keys: Vec<UncompressedPublicKey>,
    chain_spec: &ChainSpec,
) -> Result<RecoveredBlock<Block>, StatelessValidationError>
where
    ChainSpec: EthereumHardforks,
{
    if block.body().transactions.len() != public_keys.len() {
        return Err(StatelessValidationError::Custom(
            "Number of public keys must match number of transactions",
        ));
    }

    // Determine if we're in the Homestead fork for signature validation
    let is_homestead = chain_spec.is_homestead_active_at_block(block.header().number());

    // Verify each transaction signature against its corresponding public key
    let senders = public_keys
        .iter()
        .zip(block.body().transactions())
        .map(|(vk, tx)| verify_and_compute_sender(vk, tx, is_homestead))
        .collect::<Result<Vec<_>, _>>()?;

    // Create RecoveredBlock with verified senders
    let block_hash = block.hash_slow();
    Ok(RecoveredBlock::new(block, senders, block_hash))
}

/// Verifies a transaction using its signature and the given public key.
///
/// Note: If the signature or the public key is incorrect, then this method
/// will return an error.
///
/// Returns the address derived from the public key.
fn verify_and_compute_sender(
    vk: &UncompressedPublicKey,
    tx: &TransactionSigned,
    is_homestead: bool,
) -> Result<Address, StatelessValidationError> {
    verify_and_compute_sender_from_signature(vk, tx.signature(), tx.signature_hash(), is_homestead)
}

fn verify_and_compute_sender_from_signature(
    vk: &UncompressedPublicKey,
    sig: &Signature,
    sig_hash: B256,
    is_homestead: bool,
) -> Result<Address, StatelessValidationError> {
    // non-normalized signatures are only valid pre-homestead
    let sig_is_normalized = sig.normalize_s().is_none();
    if is_homestead && !sig_is_normalized {
        return Err(StatelessValidationError::HomesteadSignatureNotNormalized);
    }

    if vk.0[0] != 0x04 {
        return Err(StatelessValidationError::SignerRecovery);
    }
    let expected = Address::from_raw_public_key(&vk.0[1..]);
    let recovered = if is_homestead {
        alloy_consensus::crypto::secp256k1::recover_signer(sig, sig_hash)
    } else {
        alloy_consensus::crypto::secp256k1::recover_signer_unchecked(sig, sig_hash)
    }
    .map_err(|_| StatelessValidationError::SignerRecovery)?;

    (recovered == expected).then_some(recovered).ok_or(StatelessValidationError::SignerRecovery)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn public_key(signature: Signature, hash: B256) -> UncompressedPublicKey {
        let key = signature.recover_from_prehash(&hash).unwrap();
        UncompressedPublicKey(key.to_encoded_point(false).as_bytes().try_into().unwrap())
    }

    #[test]
    fn accepts_public_key_matching_signature_parity() {
        let hash = B256::ZERO;
        let signature = Signature::test_signature();
        let public_key = public_key(signature, hash);

        assert!(
            verify_and_compute_sender_from_signature(&public_key, &signature, hash, true).is_ok()
        );
    }

    #[test]
    fn rejects_public_key_from_opposite_signature_parity() {
        let hash = B256::ZERO;
        let signature = Signature::test_signature();
        let opposite_public_key = public_key(signature.with_parity(!signature.v()), hash);

        assert!(matches!(
            verify_and_compute_sender_from_signature(&opposite_public_key, &signature, hash, true),
            Err(StatelessValidationError::SignerRecovery)
        ));
    }
}
