# stateless

[![CI status](https://github.com/paradigmxyz/stateless/workflows/CI/badge.svg)][gh-ci]
[![Telegram Chat][tg-badge]][tg-url]

**Stateless Ethereum block validation using execution witnesses.**

[gh-ci]: https://github.com/paradigmxyz/stateless/actions/workflows/ci.yml
[tg-badge]: https://img.shields.io/endpoint?color=neon&logo=telegram&label=chat&url=https%3A%2F%2Ftg.sumanjay.workers.dev%2Fparadigm%5Freth
[tg-url]: https://t.me/paradigm_reth

## What is stateless?

`stateless` provides types and functions for validating Ethereum blocks without access to a full node's persistent database. Instead, it relies on pre-generated witness data that proves the specific state accessed during block execution.

It is built on top of [reth](https://github.com/paradigmxyz/reth) and [revm](https://github.com/bluealloy/revm), and is designed to be `no_std` compatible for use in constrained environments such as zkVMs.

## How it works

1. **Witness verification** — The execution witness is verified against the parent block's state root using a sparse Merkle Patricia trie.
2. **Block execution** — The block is executed in-memory using a witness-backed database (`WitnessDatabase`).
3. **Consensus validation** — Post-execution consensus checks are performed.
4. **State root computation** — The post-state root is calculated and compared against the block header.

## Usage

The primary entry point is `stateless_validation`:

```rust
use stateless::{stateless_validation, ExecutionWitness};

let (block_hash, output) = stateless_validation(
    block,
    public_keys,
    witness,
    chain_spec,
    evm_config,
)?;
```

## `no_std`

The `stateless` crate is `#![no_std]` compatible and builds for RISC-V targets (`riscv32imac-unknown-none-elf`), making it suitable for use in zkVM environments.

## Running EF tests

To run the Ethereum Foundation blockchain tests with stateless validation:

```bash
# From the repository root:
./scripts/run_ef_tests.sh default
./scripts/run_ef_tests.sh zeth
```

Or manually:

```bash
# From the repository root:
./scripts/setup_ef_tests.sh
EF_TEST_TRIE=default cargo test -p ef-tests --release --features "asm-keccak ef-tests"
```

## Contributing

Contributions are welcome! Join the conversation in the [Telegram group][tg-url].

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or [MIT license](LICENSE-MIT) at your option.
