use crate::validation::StatelessValidationError;
use alloc::vec::Vec;
use alloy_consensus::BlockHeader;
use alloy_primitives::{Address, B256, Signature};
use reth_chainspec::EthereumHardforks;
use reth_ethereum_primitives::{Block, TransactionSigned};
use reth_primitives_traits::{Block as _, RecoveredBlock};

/// Recovers the sender of every transaction in a block from its signature.
///
/// Returns a `RecoveredBlock`
pub fn recover_block<ChainSpec>(
    block: Block,
    chain_spec: &ChainSpec,
) -> Result<RecoveredBlock<Block>, StatelessValidationError>
where
    ChainSpec: EthereumHardforks,
{
    // Determine if we're in the Homestead fork for signature validation
    let is_homestead = chain_spec.is_homestead_active_at_block(block.header().number());

    let senders = block
        .body()
        .transactions()
        .map(|tx| recover_sender(tx, is_homestead))
        .collect::<Result<Vec<_>, _>>()?;

    let block_hash = block.hash_slow();
    Ok(RecoveredBlock::new(block, senders, block_hash))
}

/// Recovers the sender of a transaction from its signature.
fn recover_sender(
    tx: &TransactionSigned,
    is_homestead: bool,
) -> Result<Address, StatelessValidationError> {
    recover_signer(tx.signature(), tx.signature_hash(), is_homestead)
}

fn recover_signer(
    sig: &Signature,
    sig_hash: B256,
    is_homestead: bool,
) -> Result<Address, StatelessValidationError> {
    // non-normalized signatures are only valid pre-homestead
    if is_homestead && sig.normalize_s().is_some() {
        return Err(StatelessValidationError::HomesteadSignatureNotNormalized);
    }

    alloy_consensus::crypto::secp256k1::recover_signer_unchecked(sig, sig_hash)
        .map_err(|_| StatelessValidationError::SignerRecovery)
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy_primitives::{U256, hex};

    /// Order of the secp256k1 curve.
    const SECP256K1N: [u8; 32] =
        hex!("fffffffffffffffffffffffffffffffebaaedce6af48a03bbfd25e8cd0364141");

    #[test]
    fn recovers_signer_matching_signature_parity() {
        let hash = B256::ZERO;
        let signature = Signature::test_signature();
        let expected = signature.recover_address_from_prehash(&hash).unwrap();

        assert_eq!(recover_signer(&signature, hash, true).unwrap(), expected);
        assert_ne!(
            recover_signer(&signature.with_parity(!signature.v()), hash, true).unwrap(),
            expected
        );
    }

    #[test]
    fn rejects_non_normalized_signature_after_homestead() {
        let hash = B256::ZERO;
        let signature = Signature::test_signature();
        let high_s = Signature::new(
            signature.r(),
            U256::from_be_bytes(SECP256K1N) - signature.s(),
            !signature.v(),
        );
        assert!(high_s.normalize_s().is_some());

        assert!(matches!(
            recover_signer(&high_s, hash, true),
            Err(StatelessValidationError::HomesteadSignatureNotNormalized)
        ));
        assert_eq!(
            recover_signer(&high_s, hash, false).unwrap(),
            recover_signer(&signature, hash, true).unwrap()
        );
    }
}
