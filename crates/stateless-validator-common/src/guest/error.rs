//! Common errors for the stateless input guest.

use libssz::DecodeError;
use thiserror::Error;

/// Common errors for the stateless input guest.
#[derive(Debug, Error)]
pub enum Error {
    /// The input is shorter than the schema identifier prefix.
    #[error("stateless input is missing the schema id")]
    MissingSchemaId,
    /// The schema identifier prefix does not match the supported schema id.
    #[error("unsupported stateless input schema id {0:#06x}")]
    UnsupportedSchemaId(u16),
    /// The SSZ body failed to decode.
    #[error("SSZ decode error {0:?}")]
    Ssz(DecodeError),
}

impl From<DecodeError> for Error {
    fn from(err: DecodeError) -> Self {
        Self::Ssz(err)
    }
}
