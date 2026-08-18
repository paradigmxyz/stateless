//! ZisK Reth stateless validator guest program.

#![no_main]

use ere_platform_zisk::{ZiskPlatform, ziskos};
use stateless_validator_reth::guest::entrypoint;

ziskos::entrypoint!(main);

fn main() {
    entrypoint::<ZiskPlatform>();
}

#[unsafe(no_mangle)]
fn _critical_section_1_0_acquire() -> u64 {
    0
}

#[unsafe(no_mangle)]
fn _critical_section_1_0_release(_: u64) {}
