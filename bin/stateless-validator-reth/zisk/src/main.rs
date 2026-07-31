//! ZisK Reth stateless validator guest program.

#![no_main]

use ere_platform_zisk::{ZiskPlatform, ziskos};
use stateless_validator_reth::guest::{crypto::zkvm_interface, entrypoint};

ziskos::entrypoint!(main);

fn main() {
    zkvm_interface::install_crypto();

    entrypoint::<ZiskPlatform>();
}

#[unsafe(no_mangle)]
fn _critical_section_1_0_acquire() -> u64 {
    0
}

#[unsafe(no_mangle)]
fn _critical_section_1_0_release(_: u64) {}
