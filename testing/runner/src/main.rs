//! Command-line interface for running tests.
use std::path::PathBuf;

use clap::Parser;
use ef_tests::{Suite, cases::blockchain_test::BlockchainTests};
use flate2::read::GzDecoder;
use tar::Archive;
use tempfile::TempDir;

/// Command-line arguments for the test runner.
#[derive(Debug, Parser)]
pub struct TestRunnerCommand {
    /// Path to the test suite (local directory or URL to a .tar.gz archive)
    suite_path: String,
}

/// Resolve the suite path to a local directory.
///
/// If the input is a URL, download and extract the `.tar.gz` archive to a temporary directory.
/// The returned [`TempDir`] (if any) must be kept alive for the duration of the test run.
fn resolve_suite_path(input: &str) -> (PathBuf, Option<TempDir>) {
    if input.starts_with("http://") || input.starts_with("https://") {
        eprintln!("Downloading {input}...");
        let response = reqwest::blocking::get(input)
            .unwrap_or_else(|e| panic!("failed to download {input}: {e}"));

        if !response.status().is_success() {
            panic!("failed to download {input}: HTTP {}", response.status());
        }

        let temp_dir = TempDir::new().expect("failed to create temp directory");
        eprintln!("Extracting to {}...", temp_dir.path().display());

        let decoder = GzDecoder::new(response);
        let mut archive = Archive::new(decoder);
        archive.unpack(temp_dir.path()).expect("failed to extract archive");

        let path = temp_dir.path().to_path_buf();
        (path.join("fixtures"), Some(temp_dir))
    } else {
        (PathBuf::from(input), None)
    }
}

fn main() {
    let cmd = TestRunnerCommand::parse();
    let (suite_path, _temp_dir) = resolve_suite_path(&cmd.suite_path);
    BlockchainTests::new(suite_path.join("blockchain_tests")).run();
}
