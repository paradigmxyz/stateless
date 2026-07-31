use openvm_sha2::{Digest, Sha256};
use stateless_validator_common::Sha256Hasher;

/// OpenVM SHA-256 provider for SSZ tree hashing.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct OpenVMSha256Hasher;

impl Sha256Hasher for OpenVMSha256Hasher {
    fn hash(&self, input: &[u8]) -> [u8; 32] {
        Sha256::digest(input).into()
    }
}
