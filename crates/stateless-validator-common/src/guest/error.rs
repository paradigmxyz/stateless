//! Common errors for the stateless input guest.

use libssz::DecodeError;
use thiserror::Error;

use crate::guest::input::ProtocolFork;

/// Common errors for the stateless input guest.
#[derive(Debug, Error)]
pub enum Error {
    /// The input is shorter than the schema identifier prefix.
    #[error("stateless input is missing the schema id")]
    MissingSchemaId,
    /// The schema identifier prefix does not match the supported schema id.
    #[error("unsupported stateless input schema id {0:#06x}")]
    UnsupportedSchemaId(u16),
    /// The protocol fork is not supported.
    #[error("unsupported protocol fork {0:?}")]
    UnsupportedProtocolFork(ProtocolFork),
    /// The SSZ body failed to decode.
    #[error("SSZ decode error {0:?}")]
    Ssz(DecodeError),
    /// The fork activation has neither block_number nor timestamp set, mirroring the spec
    /// `InvalidForkActivationError`.
    #[error("Fork activation must set block_number or timestamp")]
    InvalidForkActivation,
    /// The configured active fork is not active for the payload, mirroring the spec
    /// `InactiveForkConfigError`.
    #[error("ChainConfig active_fork is not active for the target payload")]
    InactiveForkConfig,
}

impl From<DecodeError> for Error {
    fn from(err: DecodeError) -> Self {
        Self::Ssz(err)
    }
}
