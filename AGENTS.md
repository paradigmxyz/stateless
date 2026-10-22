# AGENTS.md

Guidance for AI coding agents working in this repository. `README.md` describes the
validation flow and the test suites, and `docs/guest-releases.md` describes guest
builds and releases. Read the latter before changing anything under
`bin/stateless-validator-reth/` or `scripts/guests/`.

## Ownership

Make each change in the crate that owns the behavior.

- `crates/stateless`: core stateless validation (`stateless_validation`,
  `WitnessDatabase`, block recovery, and the zkVM interface).
- `crates/tries`: the `tries::StatelessTrie` trait and its backends (`default` on
  `reth-trie-sparse`, `zeth` on `crates/zeth-mpt`). Put trie behavior behind this
  trait; do not special-case a backend in core validation.
- `crates/stateless-validator-common` and `crates/stateless-validator-reth`: shared
  guest logic. The common crate owns the execution-specs guest input and output
  types; the Reth crate owns the guest entrypoint, conversions, and crypto.
- `bin/stateless-validator-reth/{openvm,sp1,zisk}`: thin zkVM shims that call
  `stateless_validator_reth::guest::entrypoint`. Do not add logic or duplicate
  shared code per zkVM.
- `testing/ef-tests` and `testing/stateless-validator`: EF blockchain tests and
  execution-spec fixture tests.

## Rules

- Use collections with deterministic iteration: `IndexMap`, `B256IndexMap`, or
  `BTreeMap`, and their set equivalents. `clippy.toml` denies std, `hashbrown`,
  and `alloy_primitives::map` hash maps and sets.
- Keep the validation and guest crates building without default features for the
  RISC-V target in `targets/`. `./scripts/check_no_std.sh` checks this.
- Run Cargo with `--locked`. Change `Cargo.lock` only for an intended dependency
  update.
- The guest packages under `bin/stateless-validator-reth/` are independent Cargo
  workspaces with committed lockfiles. Leave those lockfiles unchanged unless the
  change updates guest dependencies; a guest build fails if its lockfile changes.
- The fixture tests pin `tests-zkevm@v21.0.6` through the URL and SHA-256 in
  `testing/stateless-validator/src/fixture.rs`. When updating the version, update
  the checksum, `README.md`, and the execution-specs links in the guest crates
  together.
- `KNOWN_FAILURES` in `testing/stateless-validator/tests/host_execution.rs` lists
  fixtures expected to fail, with the reason above it. The suite fails when a
  listed fixture passes, so remove entries that start passing. Add an entry only
  with its upstream cause.

## Checks

CI runs these commands:

- `cargo fmt --all --check` (stable)
- `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`
- `./scripts/run_ef_tests.sh default|zeth` (uses `cargo nextest`; CI allows the
  `zeth` run to fail)
- `./scripts/check_no_std.sh` (nightly with `rust-src`)
- `cargo test -p stateless-validator-tests --test host_execution --locked`

Run the checks that cover the changed crates. The EF and fixture suites download
test data on first use.
