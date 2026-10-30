//! Execution-spec fixture tests for the Reth guest on the host.

use stateless_validator_tests::{
    execution::{ExecutionFailures, run_host_execution},
    fixture::{FixtureFormat, eest_fixtures},
};

/// Tests whose fixtures fail, matched by test name or, where only some cases fail, by test ID.
///
/// The EIP-7928 test fails because reth-evm-ethereum runs the post-execution system calls before it
/// credits withdrawals, the reverse of EELS `apply_body`.
///
/// EIP-8025 tests omit code from the witness when the block wrote the same code before reading it,
/// through a CREATE or an EIP-7702 delegation earlier in the same transaction. EELS serves such
/// reads from the block's code writes. evm2 only does so for code written by an earlier
/// transaction, so the read goes to the witness, which lacks the code, and the guest rejects the
/// block.
const KNOWN_FAILURES: &[&str] = &[
    // EIP-7928
    "test_bal_withdrawals_and_dequeues_net_balance_at_last_index[",
    // EIP-8025
    "test_witness_codes_create_same_hash_then_read_same_tx[",
    "test_witness_codes_delegation_same_marker_cleared_then_read[fork_Amsterdam-blockchain_test-clear_in_same_tx]",
    "test_witness_codes_delegation_same_marker_cleared_then_read[fork_Amsterdam-blockchain_test_engine-clear_in_same_tx]",
    "test_witness_codes_delegation_same_marker_then_read_same_tx[",
];

fn is_known_failure(name: &str) -> bool {
    KNOWN_FAILURES.iter().any(|test| name.contains(test))
}

fn executes_eest_fixtures(format: FixtureFormat) {
    let fixtures = eest_fixtures(format);
    assert!(!fixtures.is_empty(), "no {format:?} stateless validator fixtures loaded");
    let known_failures = fixtures.iter().filter(|fixture| is_known_failure(&fixture.name)).count();

    println!("Executing {} {format:?} stateless validator fixtures", fixtures.len());
    let (known, unexpected): (Vec<_>, Vec<_>) = run_host_execution(fixtures)
        .into_iter()
        .partition(|failure| is_known_failure(&failure.name));
    assert!(
        unexpected.is_empty(),
        "stateless validator fixture failures:\n{}",
        ExecutionFailures(&unexpected),
    );
    assert_eq!(
        known.len(),
        known_failures,
        "known failures now pass, remove them from KNOWN_FAILURES"
    );
}

#[test]
fn executes_eest_blockchain_test_fixtures() {
    executes_eest_fixtures(FixtureFormat::BlockchainTest);
}

#[test]
fn executes_eest_blockchain_test_engine_fixtures() {
    executes_eest_fixtures(FixtureFormat::BlockchainTestEngine);
}
