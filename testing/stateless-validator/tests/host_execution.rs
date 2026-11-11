//! Execution-spec fixture tests for the Reth guest on the host.

use stateless_validator_tests::{
    execution::{ExecutionFailures, run_host_execution},
    fixture::{FixtureFormat, eest_fixtures},
};

/// Tests whose fixtures fail, matched by test name or, where only some cases fail, by test ID.
///
/// EIP-7685 and EIP-7928 tests fail on upstream alloy-evm and revm bugs:
///
/// - alloy-evm runs the post-execution system calls before it credits withdrawals, the reverse of
///   EELS `apply_body`.
/// - revm's BAL builder records a change when a value returns to its start-of-index value within
///   one block access index. EELS records no change, or a storage read.
/// - revm's BAL builder drops a change made earlier in a block access index when a later system
///   call in that index touches the account without changing the value.
///
/// EIP-8025 tests omit code from the witness when the block wrote the same code before reading it,
/// through a CREATE earlier in the transaction or an EIP-7702 delegation earlier in the block.
/// EELS serves such reads from the block's code writes, but the guest reads code only from the
/// witness and rejects the block.
const KNOWN_FAILURES: &[&str] = &[
    // EIP-7685
    "test_withdrawals_credited_before_request_system_call[",
    // EIP-7928
    "test_bal_post_execution_calls_net_storage_at_last_index[",
    "test_bal_same_index_call_keeps_one_nonce_and_code_change[",
    "test_bal_withdrawals_and_dequeues_net_balance_at_last_index[",
    // EIP-8025
    "test_witness_codes_create_same_hash_then_read_same_tx[",
    "test_witness_codes_delegation_same_marker_cleared_then_read[",
    "test_witness_codes_delegation_same_marker_then_read[",
    "test_witness_codes_delegation_same_marker_then_read_same_tx[",
    "test_witness_codes_delegation_same_marker_tx_outcome[fork_Amsterdam-blockchain_test-outcome_execution_reverts]",
    "test_witness_codes_delegation_same_marker_tx_outcome[fork_Amsterdam-blockchain_test-outcome_preparation_exact_gas]",
    "test_witness_codes_delegation_same_marker_tx_outcome[fork_Amsterdam-blockchain_test_engine-outcome_execution_reverts]",
    "test_witness_codes_delegation_same_marker_tx_outcome[fork_Amsterdam-blockchain_test_engine-outcome_preparation_exact_gas]",
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
