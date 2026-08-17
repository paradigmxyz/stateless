//! Crypto provider selection for the guest.

#[cfg(feature = "sp1")]
pub(crate) mod sp1;

use stateless_validator_common::Sha256Hasher;

#[cfg(feature = "zkvm-interface")]
pub(crate) use stateless::zkvm_interface::install_crypto;

#[cfg(feature = "zkvm-interface")]
#[derive(Debug, Clone, Copy, Default)]
struct ZkVMInterfaceSha256;

#[cfg(feature = "zkvm-interface")]
impl Sha256Hasher for ZkVMInterfaceSha256 {
    #[inline]
    fn hash(&self, data: &[u8]) -> [u8; 32] {
        stateless::zkvm_interface::sha256(data)
    }
}

/// Returns the [`Sha256Hasher`] implementation for the active zkVM feature.
#[cfg(feature = "zkvm-interface")]
pub(crate) fn sha256_hasher() -> impl Sha256Hasher {
    ZkVMInterfaceSha256
}

#[cfg(not(feature = "zkvm-interface"))]
pub(crate) fn sha256_hasher() -> impl Sha256Hasher {
    stateless_validator_common::Sha2Hasher
}
