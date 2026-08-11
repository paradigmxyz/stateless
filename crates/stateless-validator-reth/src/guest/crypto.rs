//! Crypto provider selection for the guest.

#[cfg(feature = "sp1")]
pub(crate) mod sp1;
#[cfg(feature = "zkvm-interface")]
pub mod zkvm_interface;

use stateless_validator_common::Sha256Hasher;

/// Returns the [`Sha256Hasher`] implementation for the active zkVM feature.
#[cfg(feature = "zkvm-interface")]
pub(crate) fn sha256_hasher() -> impl Sha256Hasher {
    zkvm_interface::sha256_hasher()
}

#[cfg(not(feature = "zkvm-interface"))]
pub(crate) fn sha256_hasher() -> impl Sha256Hasher {
    stateless_validator_common::Sha2Hasher
}
