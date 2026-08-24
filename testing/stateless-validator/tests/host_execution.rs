//! Execution-spec fixture tests for the Reth guest on the host.

use stateless_validator_tests::{
    execution::{ExecutionFailures, run_host_execution},
    fixture::eest_fixtures,
};

const EXPECTED_FAILURES: &[&str] = &[
    // Reth/revm does not index bytecode created earlier in the block by code hash. This fixture
    // correctly omits that bytecode from the witness under EIP-8025, then reads the same hash from
    // another account in a later transaction.
    "tests/amsterdam/eip8025_optional_proofs/test_witness_bytecodes_contract_creation.py::test_witness_codes_create_same_hash_then_read[fork_Amsterdam-blockchain_test]#block0",
    "tests/paris/eip7610_create_collision/test_initcollision.py::test_init_collision_create_opcode[fork_Amsterdam-blockchain_test_from_state_test-opcode_CREATE-non-empty-balance-correct-initcode]#block0",
    "tests/paris/eip7610_create_collision/test_initcollision.py::test_init_collision_create_opcode[fork_Amsterdam-blockchain_test_from_state_test-opcode_CREATE2-non-empty-balance-correct-initcode]#block0",
    "tests/paris/eip7610_create_collision/test_initcollision.py::test_init_collision_create_tx[fork_Amsterdam-tx_type_0-blockchain_test_from_state_test-non-empty-balance-correct-initcode]#block0",
    "tests/paris/eip7610_create_collision/test_initcollision.py::test_init_collision_create_tx[fork_Amsterdam-tx_type_0-blockchain_test_from_state_test-non-empty-balance-revert-initcode]#block0",
    "tests/paris/eip7610_create_collision/test_initcollision.py::test_init_collision_create_tx[fork_Amsterdam-tx_type_1-blockchain_test_from_state_test-non-empty-balance-correct-initcode]#block0",
    "tests/paris/eip7610_create_collision/test_initcollision.py::test_init_collision_create_tx[fork_Amsterdam-tx_type_1-blockchain_test_from_state_test-non-empty-balance-revert-initcode]#block0",
    "tests/paris/eip7610_create_collision/test_initcollision.py::test_init_collision_create_tx[fork_Amsterdam-tx_type_2-blockchain_test_from_state_test-non-empty-balance-correct-initcode]#block0",
    "tests/paris/eip7610_create_collision/test_initcollision.py::test_init_collision_create_tx[fork_Amsterdam-tx_type_2-blockchain_test_from_state_test-non-empty-balance-revert-initcode]#block0",
    "tests/paris/eip7610_create_collision/test_revert_in_create.py::test_collision_with_create2_revert_in_initcode[fork_Amsterdam-blockchain_test_from_state_test]#block0",
    "tests/paris/eip7610_create_collision/test_revert_in_create.py::test_create2_collision_storage[fork_Amsterdam-blockchain_test_from_state_test-empty-initcode]#block0",
    "tests/paris/eip7610_create_collision/test_revert_in_create.py::test_create2_collision_storage[fork_Amsterdam-blockchain_test_from_state_test-initcode-with-deploy]#block0",
    "tests/paris/eip7610_create_collision/test_revert_in_create.py::test_create2_collision_storage[fork_Amsterdam-blockchain_test_from_state_test-sstore-initcode]#block0",
];

#[test]
fn executes_eest_glamsterdam_fixtures() {
    let failures = run_host_execution(eest_fixtures());
    let failure_names = failures.iter().map(|failure| failure.name.as_str()).collect::<Vec<_>>();
    assert_eq!(
        failure_names,
        EXPECTED_FAILURES,
        "unexpected upstream Reth failure set ({} failures):\n{}",
        failures.len(),
        ExecutionFailures(&failures),
    );
}
