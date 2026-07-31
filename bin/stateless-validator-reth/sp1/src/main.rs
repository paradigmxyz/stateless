//! SP1 Reth stateless validator guest program.

#![no_main]

use ere_platform_sp1::{SP1Platform, sp1_zkvm};
use stateless_validator_reth::guest::{crypto::zkvm_interface, entrypoint};

sp1_zkvm::entrypoint!(main);

fn main() {
    zkvm_interface::install_crypto();
    entrypoint::<SP1Platform>();
}
