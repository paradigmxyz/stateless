#![allow(missing_docs)]
#![cfg(feature = "ef-tests")]

use ef_tests::{cases::blockchain_test::BlockchainTests, suite::Suite};
use std::path::PathBuf;

macro_rules! blockchain_test {
    ($test_name:ident, $dir:ident) => {
        #[test]
        fn $test_name() {
            reth_tracing::init_test_tracing();
            let suite_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("ethereum-tests")
                .join("BlockchainTests");

            BlockchainTests::new(suite_path, Default::default()).run_only(stringify!($dir));
        }
    };
}

blockchain_test!(valid_blocks, ValidBlocks);
blockchain_test!(invalid_blocks, InvalidBlocks);

#[test]
fn eest_fixtures() {
    reth_tracing::init_test_tracing();
    let suite_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("execution-spec-tests")
        .join("for_amsterdam");

    if !suite_path.exists() {
        return;
    }

    BlockchainTests::new(suite_path, Default::default()).run();
}
