//! Errors for the stateless input guest.

use alloy_rpc_types_engine::PayloadError;
use stateless::validation::StatelessValidationError;
use thiserror::Error;

/// Errors for the stateless input guest. Each variant tags the point at which
/// conversion or validation fails rather than carrying diagnostic detail.
#[derive(Debug, Error)]
pub enum Error {
    /// Shared guest validation failed.
    #[error(transparent)]
    Common(#[from] stateless_validator_common::guest::Error),
    /// The payload was not well formed.
    #[error(transparent)]
    PayloadError(#[from] PayloadError),
    /// The reth execution path rejected the payload.
    #[error(transparent)]
    StatelessValidationError(#[from] StatelessValidationError),
}
