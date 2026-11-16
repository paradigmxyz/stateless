//! Pico Reth stateless validator guest program.

#![no_main]

use ere_platform_pico::{PicoPlatform, pico_sdk};
use stateless_validator_reth::guest::entrypoint;

pico_sdk::entrypoint!(main);

fn main() {
    entrypoint::<PicoPlatform>();
}
