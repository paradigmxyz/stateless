//! Crypto provider selection for the guest.

#[cfg(feature = "openvm")]
pub(crate) mod openvm;
#[cfg(feature = "sp1")]
pub(crate) mod sp1;
#[cfg(feature = "zkvm-interface")]
pub mod zkvm_interface;

use stateless_validator_common::Sha256Hasher;

/// Returns the [`Sha256Hasher`] implementation for the active zkVM feature.
#[allow(unreachable_code)]
pub(crate) fn sha256_hasher() -> impl Sha256Hasher {
    #[cfg(feature = "openvm")]
    return openvm::OpenVMSha256Hasher;
    #[cfg(all(not(feature = "openvm"), feature = "zkvm-interface"))]
    return zkvm_interface::sha256_hasher();
    #[cfg(not(any(feature = "openvm", feature = "zkvm-interface")))]
    return stateless_validator_common::Sha2Hasher;
}
