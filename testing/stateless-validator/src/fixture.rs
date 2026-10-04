//! Fixture loading for the stateless validator.

use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    sync::Mutex,
};

use alloy_primitives::Bytes;
use rayon::prelude::*;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use tar::Archive;
use walkdir::{DirEntry, WalkDir};

const EEST_FIXTURES_URL: &str = "https://github.com/ethereum/execution-specs/releases/download/tests-zkevm@v21.0.1/fixtures_zkevm.tar.gz";
const EEST_FIXTURES_SHA256: &str =
    "42fa627e2e262b109ce29d255fb9c11baa297246f759c6edaf0e58f58d4aa205";

/// A fixture normalized to canonical schema-prefixed SSZ input and output bytes.
#[derive(Debug, Clone)]
pub struct StatelessValidatorFixture {
    /// Human-readable identifier.
    pub name: String,
    /// Canonical schema-prefixed SSZ input bytes consumed by the guest.
    pub stateless_input_bytes: Vec<u8>,
    /// Expected serialized guest output bytes.
    pub stateless_output_bytes: Vec<u8>,
}

/// Execution-spec fixture formats that carry stateless guest inputs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FixtureFormat {
    /// `blockchain_test` fixtures, with stateless bytes on each RLP block.
    BlockchainTest,
    /// `blockchain_test_engine` fixtures, with stateless bytes on each engine API payload.
    BlockchainTestEngine,
}

impl FixtureFormat {
    /// Directory of this format in the fixture archive.
    const fn dir_name(self) -> &'static str {
        match self {
            Self::BlockchainTest => "blockchain_tests",
            Self::BlockchainTestEngine => "blockchain_tests_engine",
        }
    }
}

/// Returns all `tests-zkevm@v21.0.1` fixtures of the given format, downloading them on first use.
pub fn eest_fixtures(format: FixtureFormat) -> Vec<StatelessValidatorFixture> {
    load_fixtures_from_dir(ensure_eest_fixtures().join(format.dir_name()), format)
}

fn is_json_file(entry: &DirEntry) -> bool {
    entry.file_type().is_file()
        && entry
            .path()
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.ends_with(".json"))
}

fn load_fixtures_from_dir(
    dir: impl AsRef<Path>,
    format: FixtureFormat,
) -> Vec<StatelessValidatorFixture> {
    let mut fixtures = WalkDir::new(dir)
        .into_iter()
        .par_bridge()
        .filter_map(Result::ok)
        .filter(is_json_file)
        .flat_map(|entry| load_fixtures_from_file(entry.path(), format))
        .collect::<Vec<_>>();
    fixtures.sort_by(|a, b| a.name.cmp(&b.name));
    fixtures
}

fn load_fixtures_from_file(
    path: impl AsRef<Path>,
    format: FixtureFormat,
) -> Vec<StatelessValidatorFixture> {
    let bytes = fs::read(path).unwrap();
    let tests: EestFixture = serde_json::from_slice(&bytes).unwrap();
    tests
        .into_iter()
        .flat_map(|(test_id, test)| {
            let (label, blocks) = match format {
                FixtureFormat::BlockchainTest => ("block", test.blocks),
                FixtureFormat::BlockchainTestEngine => ("payload", test.engine_new_payloads),
            };
            blocks.into_iter().enumerate().filter_map(move |(idx, block)| {
                let (input, output) =
                    block.stateless_input_bytes.zip(block.stateless_output_bytes)?;
                (!input.is_empty()).then(|| StatelessValidatorFixture {
                    name: format!("{test_id}#{label}{idx}"),
                    stateless_input_bytes: input.to_vec(),
                    stateless_output_bytes: output.to_vec(),
                })
            })
        })
        .collect()
}

fn ensure_eest_fixtures() -> PathBuf {
    static LOCK: Mutex<()> = Mutex::new(());
    let _guard = LOCK.lock().unwrap_or_else(|err| err.into_inner());

    let dir =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fixtures").join("eest-tests-zkevm-v21.0.1");
    if !dir.exists() {
        download_and_unpack(&dir);
    }
    dir
}

fn download_and_unpack(dir: &Path) {
    let bytes = reqwest::blocking::get(EEST_FIXTURES_URL)
        .unwrap()
        .error_for_status()
        .unwrap()
        .bytes()
        .unwrap();
    assert_eq!(
        const_hex::encode(Sha256::digest(&bytes)),
        EEST_FIXTURES_SHA256,
        "fixture archive checksum mismatch"
    );

    fs::create_dir_all(dir.parent().unwrap()).unwrap();
    let tempdir = tempfile::tempdir_in(dir.parent().unwrap()).unwrap();
    Archive::new(flate2::read::GzDecoder::new(&bytes[..])).unpack(tempdir.path()).unwrap();
    fs::rename(tempdir.path().join("fixtures"), dir).unwrap();
}

type EestFixture = BTreeMap<String, EestTest>;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct EestTest {
    #[serde(default)]
    blocks: Vec<EestBlock>,
    #[serde(default)]
    engine_new_payloads: Vec<EestBlock>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct EestBlock {
    stateless_input_bytes: Option<Bytes>,
    stateless_output_bytes: Option<Bytes>,
}
