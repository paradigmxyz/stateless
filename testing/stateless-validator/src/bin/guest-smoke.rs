//! Executes a built guest artifact against the release smoke fixture.

use std::{env, fs, path::PathBuf, time::Duration};

use alloy_primitives::Bytes;
use anyhow::{Context, Result, anyhow, ensure};
use ere_server_client::{Input, reqwest::Client, url::Url, zkVMClient};
use ere_util_tokio::block_on;
use serde::Deserialize;
use stateless_validator_tests::execution::matches_output;

const REQUEST_TIMEOUT: Duration = Duration::from_secs(15 * 60);

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SmokeFixture {
    name: String,
    stateless_input_bytes: Bytes,
    stateless_output_bytes: Bytes,
}

fn next_path(args: &mut impl Iterator<Item = String>, name: &str) -> Result<PathBuf> {
    args.next().map(PathBuf::from).ok_or_else(|| anyhow!("missing {name} argument"))
}

fn main() -> Result<()> {
    let mut args = env::args().skip(1);
    let zkvm_name = args.next().ok_or_else(|| anyhow!("missing zkVM argument"))?;
    let endpoint = args.next().ok_or_else(|| anyhow!("missing server endpoint argument"))?;
    let vk_path = next_path(&mut args, "verification key")?;
    let fixture_path = next_path(&mut args, "fixture")?;
    ensure!(args.next().is_none(), "unexpected extra arguments");

    let fixture: SmokeFixture = serde_json::from_slice(
        &fs::read(&fixture_path)
            .with_context(|| format!("failed to read {}", fixture_path.display()))?,
    )?;
    let expected_vk =
        fs::read(&vk_path).with_context(|| format!("failed to read {}", vk_path.display()))?;
    let http_client = Client::builder().no_proxy().timeout(REQUEST_TIMEOUT).build()?;
    let zkvm = zkVMClient::new(Url::parse(&endpoint)?, http_client, vec![])?;

    ensure!(
        block_on(zkvm.program_vk())?.0.as_slice() == expected_vk.as_slice(),
        "regenerated program VK differs from {}",
        vk_path.display()
    );
    let output =
        block_on(zkvm.execute(Input::new().with_stdin(fixture.stateless_input_bytes.to_vec())))?
            .0
            .to_vec();
    matches_output(output, fixture.stateless_output_bytes.to_vec())
        .with_context(|| format!("{} failed on {zkvm_name}", fixture.name))?;

    println!("{} passed on {zkvm_name}", fixture.name);
    Ok(())
}
