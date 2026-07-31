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
use tracing::info;
use walkdir::{DirEntry, WalkDir};

const EEST_FIXTURES_URL: &str = "https://github.com/ethereum/execution-specs/releases/download/tests-zkevm@v0.6.2/fixtures_zkevm.tar.gz";
const EEST_FIXTURES_SHA256: &str =
    "cf9395b2cb1a87c195fd827ea03dce65a37c6036bb182441da28e7b0f6d45f40";

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

/// Returns all `tests-zkevm@v0.6.2` fixtures, downloading them on first use.
pub fn eest_fixtures() -> Vec<StatelessValidatorFixture> {
    load_fixtures_from_dir(ensure_eest_fixtures())
}

fn is_json_file(entry: &DirEntry) -> bool {
    entry.file_type().is_file()
        && entry
            .path()
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.ends_with(".json"))
}

fn load_fixtures_from_dir(dir: impl AsRef<Path>) -> Vec<StatelessValidatorFixture> {
    let mut fixtures = WalkDir::new(dir)
        .into_iter()
        .par_bridge()
        .filter_map(Result::ok)
        .filter(is_json_file)
        .flat_map(|entry| load_fixtures_from_file(entry.path()))
        .collect::<Vec<_>>();
    fixtures.sort_by(|a, b| a.name.cmp(&b.name));
    fixtures
}

fn load_fixtures_from_file(path: impl AsRef<Path>) -> Vec<StatelessValidatorFixture> {
    let bytes = fs::read(path).unwrap();
    let tests: EestFixture = serde_json::from_slice(&bytes).unwrap();
    tests
        .into_iter()
        .flat_map(|(test_id, test)| {
            test.blocks.into_iter().enumerate().filter_map(move |(idx, block)| {
                let (input, output) =
                    block.stateless_input_bytes.zip(block.stateless_output_bytes)?;
                (!input.is_empty()).then(|| StatelessValidatorFixture {
                    name: format!("{test_id}#block{idx}"),
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

    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("fixtures")
        .join("eest-glamsterdam-devnet-7");
    if !dir.exists() {
        download_and_unpack(&dir);
    }
    dir
}

fn download_and_unpack(dir: &Path) {
    info!("Downloading fixture archive {EEST_FIXTURES_URL}");
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
    fs::rename(tempdir.path().join("fixtures/blockchain_tests"), dir).unwrap();
}

type EestFixture = BTreeMap<String, EestTest>;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct EestTest {
    blocks: Vec<EestBlock>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct EestBlock {
    stateless_input_bytes: Option<Bytes>,
    stateless_output_bytes: Option<Bytes>,
}
