//! SSZ hashing adapter for the standard zkVM crypto provider.

use revm::precompile::Crypto;
pub use stateless::zkvm_interface::{ZkVMInterfaceCrypto, install_crypto};
use stateless_validator_common::Sha256Hasher;

/// Returns a [`Sha256Hasher`] backed by the standard zkVM interface.
#[inline]
pub fn sha256_hasher() -> impl Sha256Hasher {
    ZkVMInterfaceSha256
}

#[derive(Debug, Clone, Copy, Default)]
struct ZkVMInterfaceSha256;

impl Sha256Hasher for ZkVMInterfaceSha256 {
    #[inline]
    fn hash(&self, data: &[u8]) -> [u8; 32] {
        ZkVMInterfaceCrypto.sha256(data)
    }
}
