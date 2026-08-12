//! Execution-spec fixture tests for the Reth guest on the host.

use stateless_validator_tests::{
    execution::{ExecutionFailures, run_host_execution},
    fixture::eest_fixtures,
};

#[test]
fn executes_eest_glamsterdam_fixtures() {
    let failures = run_host_execution(eest_fixtures());
    assert!(
        failures.is_empty(),
        "{} execution failures:\n{}",
        failures.len(),
        ExecutionFailures(&failures),
    );
}
