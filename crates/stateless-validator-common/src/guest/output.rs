//! Canonical stateless validation output types.
//!
//! The types mirror `StatelessValidationResult` in [`stateless.py`] and its SSZ schema in
//! [`stateless_ssz.py`]. The serialized form is the plain SSZ encoding without a schema prefix.
//!
//! [`stateless.py`]: https://github.com/ethereum/execution-specs/blob/tests-zkevm@v0.6.2/src/ethereum/forks/amsterdam/stateless.py
//! [`stateless_ssz.py`]: https://github.com/ethereum/execution-specs/blob/tests-zkevm@v0.6.2/src/ethereum/forks/amsterdam/stateless_ssz.py

use alloc::vec::Vec;

use libssz_derive::{SszDecode, SszEncode};

use crate::guest::input::ChainConfig;

/// Canonical result returned by stateless validation.
#[derive(Debug, Clone, Default, PartialEq, Eq, SszEncode, SszDecode)]
pub struct StatelessValidationResult {
    /// The SSZ hash tree root of the validated payload request.
    pub new_payload_request_root: [u8; 32],
    /// Whether the stateless validation succeeded.
    pub successful_validation: bool,
    /// The chain configuration echoed from the decoded input.
    pub chain_config: ChainConfig,
}
