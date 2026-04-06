//! Command-line interface for running tests.
use std::{collections::HashSet, fs, path::PathBuf};

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

    /// Optional filter: only run test cases whose path contains this substring
    #[arg(long)]
    filter: Option<String>,

    /// Path to a file containing test file names to skip (one per line).
    /// Lines starting with '#' and empty lines are ignored.
    #[arg(long)]
    skip: Option<PathBuf>,
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

    let skip_tests = match &cmd.skip {
        Some(skip_path) => {
            let content = fs::read_to_string(skip_path).unwrap_or_else(|e| {
                panic!("failed to read skip file {}: {e}", skip_path.display())
            });
            content
                .lines()
                .map(str::trim)
                .filter(|line| !line.is_empty() && !line.starts_with('#'))
                .map(String::from)
                .collect()
        }
        None => HashSet::new(),
    };

    let (suite_path, _temp_dir) = resolve_suite_path(&cmd.suite_path);
    let tests = BlockchainTests::new(suite_path.join("blockchain_tests"), skip_tests);
    if let Some(filter) = cmd.filter {
        tests.run_with_filter(&filter);
    } else {
        tests.run();
    }
}
