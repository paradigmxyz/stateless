//! Host-side execution of stateless validator fixtures.

use std::{
    fmt::{self, Display},
    io::{self, Write},
    sync::Once,
};

use anyhow::{anyhow, bail};
use ere_platform_core::Platform;
use rayon::prelude::*;
use stateless_validator_common::{SszDecode, guest::StatelessValidationResult};
use tracing_subscriber::EnvFilter;

use crate::fixture::StatelessValidatorFixture;

/// A platform for running guest logic directly on the host.
#[derive(Debug)]
pub struct HostPlatform;

impl Platform for HostPlatform {
    #[allow(unreachable_code)]
    fn read_input() -> impl std::ops::Deref<Target = [u8]> {
        unreachable!("host tests call run_stateless_guest directly");
        &[] as &[u8]
    }

    fn write_output(_: &[u8]) {
        unreachable!("host tests call run_stateless_guest directly");
    }

    fn print(message: &str) {
        print!("{message}");
        let _ = io::stdout().flush();
    }
}

/// A fixture whose output differed from the execution-spec result.
#[derive(Debug, Clone)]
pub struct ExecutionFailure {
    /// Fixture identifier.
    pub name: String,
    /// Output mismatch or execution error.
    pub error: String,
}

/// Display wrapper for execution failures.
#[derive(Debug)]
pub struct ExecutionFailures<'a>(pub &'a [ExecutionFailure]);

impl Display for ExecutionFailures<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "{} execution failures:", self.0.len())?;
        for failure in self.0 {
            writeln!(f, "  - {}", failure.name)?;
            writeln!(f, "    {}", failure.error)?;
        }
        Ok(())
    }
}

/// Executes fixtures through the Reth guest and returns output mismatches.
pub fn run_host_execution(
    fixtures: impl IntoIterator<Item = StatelessValidatorFixture>,
) -> Vec<ExecutionFailure> {
    static INIT_TRACING: Once = Once::new();
    INIT_TRACING.call_once(|| {
        let _ = tracing_subscriber::fmt().with_env_filter(EnvFilter::from_default_env()).try_init();
    });

    let mut failures = fixtures
        .into_iter()
        .collect::<Vec<_>>()
        .into_par_iter()
        .filter_map(|fixture| {
            let output = stateless_validator_reth::guest::run_stateless_guest::<HostPlatform>(
                &fixture.stateless_input_bytes,
            );
            matches_output(output, fixture.stateless_output_bytes)
                .err()
                .map(|error| ExecutionFailure { name: fixture.name, error: error.to_string() })
        })
        .collect::<Vec<_>>();
    failures.sort_by(|a, b| a.name.cmp(&b.name));
    failures
}

fn matches_output(got_bytes: Vec<u8>, expected_bytes: Vec<u8>) -> anyhow::Result<()> {
    let Some(got_bytes) =
        got_bytes.split_at_checked(expected_bytes.len()).and_then(|(got_bytes, trailing)| {
            trailing.iter().all(|byte| *byte == 0).then_some(got_bytes)
        })
    else {
        bail!(
            "output bytes mismatch, expected {}, got {}",
            const_hex::encode_prefixed(expected_bytes),
            const_hex::encode_prefixed(got_bytes)
        )
    };

    let got = StatelessValidationResult::from_ssz_bytes(got_bytes)
        .map_err(|error| anyhow!("failed to decode guest output: {error:?}"))?;
    let expected = StatelessValidationResult::from_ssz_bytes(&expected_bytes)
        .map_err(|error| anyhow!("failed to decode fixture output: {error:?}"))?;

    if got == expected {
        Ok(())
    } else {
        bail!("output mismatch, expected {expected:?}, got {got:?}")
    }
}
