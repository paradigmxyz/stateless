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

// OpenVM guests execute on one thread, so critical sections need no runtime synchronization.
#[unsafe(no_mangle)]
fn _critical_section_1_0_acquire() -> u64 {
    0
}

#[unsafe(no_mangle)]
fn _critical_section_1_0_release(_: u64) {}
