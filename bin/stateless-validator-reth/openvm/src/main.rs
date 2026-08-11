//! OpenVM Reth stateless validator guest program.

use ere_platform_openvm::OpenVMPlatform;
use stateless_validator_reth::guest::{crypto::zkvm_interface, entrypoint};

ere_platform_openvm::openvm::init!();

fn main() {
    zkvm_interface::install_crypto();
    entrypoint::<OpenVMPlatform>();
}
