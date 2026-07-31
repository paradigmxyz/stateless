//! Canonical stateless validation types and functions for guest programs.
//!
//! This module only republishes the items of its submodules so guests can
//! import every canonical type and function from a single path.

mod error;
pub mod input;
mod output;

pub use error::Error;
pub use input::StatelessInput;
pub use output::StatelessValidationResult;
