//! Canonical stateless validation output types.
//!
//! The types mirror `StatelessValidationResult` in [`stateless.py`] and its SSZ schema in
//! [`stateless_ssz.py`]. The serialized form is the plain SSZ encoding without a schema prefix.
//!
//! [`stateless.py`]: https://github.com/ethereum/execution-specs/blob/tests-zkevm@v0.8.2/src/ethereum/forks/amsterdam/stateless.py
//! [`stateless_ssz.py`]: https://github.com/ethereum/execution-specs/blob/tests-zkevm@v0.8.2/src/ethereum/forks/amsterdam/stateless_ssz.py

use alloc::vec::Vec;

use libssz_derive::{SszDecode, SszEncode};

/// Canonical result returned by stateless validation.
#[derive(Debug, Clone, Default, PartialEq, Eq, SszEncode, SszDecode)]
pub struct StatelessValidationResult {
    /// The SSZ hash tree root of the validated payload request.
    pub new_payload_request_root: [u8; 32],
    /// Whether the stateless validation succeeded.
    pub successful_validation: bool,
    /// The chain identifier echoed from the decoded input.
    pub chain_id: u64,
    /// The exact schema identifier decoded and executed by the guest.
    pub schema_id: u16,
}

#[cfg(test)]
mod tests {
    use libssz::{SszDecode as _, SszEncode as _};

    use super::*;

    #[test]
    fn validation_result_has_fixed_v08_layout() {
        let result = StatelessValidationResult {
            new_payload_request_root: [0xaa; 32],
            successful_validation: true,
            chain_id: 0x0102_0304_0506_0708,
            schema_id: 0x1501,
        };
        let encoded = result.to_ssz();

        assert_eq!(encoded.len(), 43);
        assert_eq!(&encoded[..32], &[0xaa; 32]);
        assert_eq!(encoded[32], 1);
        assert_eq!(&encoded[33..41], &0x0102_0304_0506_0708_u64.to_le_bytes());
        assert_eq!(&encoded[41..], &0x1501_u16.to_le_bytes());
        assert_eq!(StatelessValidationResult::from_ssz_bytes(&encoded).unwrap(), result);
    }

    #[test]
    fn default_validation_result_is_zero_sentinel() {
        assert_eq!(StatelessValidationResult::default().to_ssz(), [0; 43]);
    }
}
