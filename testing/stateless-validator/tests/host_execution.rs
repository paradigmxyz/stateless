//! Execution-spec fixture tests for the Reth guest on the host.

use stateless_validator_tests::{
    execution::{ExecutionFailures, run_host_execution},
    fixture::eest_fixtures,
};

#[test]
fn executes_eest_glamsterdam_fixtures() {
    let failures = run_host_execution(eest_fixtures());
    assert_eq!(
        failures.len(),
        17,
        "expected 17 known upstream Reth failures, got {}:\n{}",
        failures.len(),
        ExecutionFailures(&failures),
    );
}
