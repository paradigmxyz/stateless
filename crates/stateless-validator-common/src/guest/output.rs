//! Canonical stateless validation output types.
//!
//! The types mirror `StatelessValidationResult` in [`stateless.py`] and its SSZ schema in
//! [`stateless_ssz.py`]. The serialized form is the plain SSZ encoding without a schema prefix.
//!
//! [`stateless.py`]: https://github.com/ethereum/execution-specs/blob/tests-zkevm@v0.6.2/src/ethereum/forks/amsterdam/stateless.py
//! [`stateless_ssz.py`]: https://github.com/ethereum/execution-specs/blob/tests-zkevm@v0.6.2/src/ethereum/forks/amsterdam/stateless_ssz.py

use alloc::vec::Vec;
use core::fmt::{self, Debug};

use libssz_derive::{SszDecode, SszEncode};

use crate::guest::input::ChainConfig;

/// Canonical result returned by stateless validation.
#[derive(Clone, Default, PartialEq, Eq, SszEncode, SszDecode)]
pub struct StatelessValidationResult {
    /// The SSZ hash tree root of the validated payload request.
    pub new_payload_request_root: [u8; 32],
    /// Whether the stateless validation succeeded.
    pub successful_validation: bool,
    /// The chain configuration echoed from the decoded input.
    pub chain_config: ChainConfig,
}

impl StatelessValidationResult {
    /// Constructs a new [`StatelessValidationResult`].
    pub fn new(
        new_payload_request_root: [u8; 32],
        successful_validation: bool,
        chain_config: ChainConfig,
    ) -> Self {
        Self { new_payload_request_root, successful_validation, chain_config }
    }
}

impl Debug for StatelessValidationResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("StatelessValidationResult")
            .field(
                "new_payload_request_root",
                &const_hex::encode_prefixed(self.new_payload_request_root),
            )
            .field("successful_validation", &self.successful_validation)
            .field("chain_config", &self.chain_config)
            .finish()
    }
}
