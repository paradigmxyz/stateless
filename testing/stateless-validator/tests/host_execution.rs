//! Execution-spec fixture tests for the Reth guest on the host.

use stateless_validator_tests::{
    execution::{ExecutionFailures, run_host_execution},
    fixture::eest_fixtures,
};

#[test]
fn executes_eest_glamsterdam_fixtures() {
    let fixtures = eest_fixtures();
    assert!(!fixtures.is_empty(), "no stateless validator fixtures loaded");

    println!("Executing {} stateless validator fixtures", fixtures.len());
    let failures = run_host_execution(fixtures);
    assert!(
        failures.is_empty(),
        "stateless validator fixture failures:\n{}",
        ExecutionFailures(&failures),
    );
}
