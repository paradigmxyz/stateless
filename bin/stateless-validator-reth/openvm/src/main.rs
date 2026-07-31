//! OpenVM Reth stateless validator guest program.

use ere_platform_openvm::OpenVMPlatform;
use stateless_validator_reth::guest::entrypoint;

#[rustfmt::skip]
mod openvm_revm_crypto;

openvm::init!();

fn main() {
    openvm_revm_crypto::install_openvm_crypto()
        .expect("failed to install OpenVM revm crypto provider");
    entrypoint::<OpenVMPlatform>();
}
