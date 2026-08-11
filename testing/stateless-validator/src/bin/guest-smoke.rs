//! Executes a built guest artifact against the release smoke fixture.

use std::{env, fs, path::PathBuf, time::Duration};

use alloy_primitives::Bytes;
use anyhow::{Context, Result, anyhow, ensure};
use ere_dockerized::{DockerizedzkVM, DockerizedzkVMConfig, Elf, Input, ProverResource, zkVMKind};
use serde::Deserialize;
use stateless_validator_tests::execution::matches_output;

const HEALTH_TIMEOUT: Duration = Duration::from_secs(15 * 60);

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
    let zkvm_kind =
        zkvm_name.parse::<zkVMKind>().map_err(|_| anyhow!("unknown zkVM {zkvm_name}"))?;
    let elf_path = next_path(&mut args, "ELF")?;
    let vk_path = next_path(&mut args, "verification key")?;
    let fixture_path = next_path(&mut args, "fixture")?;
    ensure!(args.next().is_none(), "unexpected extra arguments");

    let fixture: SmokeFixture = serde_json::from_slice(
        &fs::read(&fixture_path)
            .with_context(|| format!("failed to read {}", fixture_path.display()))?,
    )?;
    let elf =
        Elf(fs::read(&elf_path)
            .with_context(|| format!("failed to read {}", elf_path.display()))?);
    let expected_vk =
        fs::read(&vk_path).with_context(|| format!("failed to read {}", vk_path.display()))?;
    let zkvm = DockerizedzkVM::new(
        zkvm_kind,
        elf,
        ProverResource::Cpu,
        DockerizedzkVMConfig { health_timeout: HEALTH_TIMEOUT, ..Default::default() },
    )?;

    ensure!(
        zkvm.program_vk().0.as_slice() == expected_vk.as_slice(),
        "regenerated program VK differs from {}",
        vk_path.display()
    );
    let output =
        zkvm.execute(&Input::new().with_stdin(fixture.stateless_input_bytes.to_vec()))?.0.to_vec();
    matches_output(output, fixture.stateless_output_bytes.to_vec())
        .with_context(|| format!("{} failed on {zkvm_name}", fixture.name))?;

    println!("{} passed on {zkvm_name}", fixture.name);
    Ok(())
}
