//! Various assertion helpers.

use crate::Error;
use alloy_primitives::Bytes;
use std::{collections::BTreeSet, fmt::Debug};

/// A helper like `assert_eq!` that instead returns `Err(Error::Assertion)` on failure.
pub fn assert_equal<T>(left: T, right: T, msg: &str) -> Result<(), Error>
where
    T: PartialEq + Debug,
{
    if left == right {
        Ok(())
    } else {
        Err(Error::Assertion(format!("{msg}\n  left `{left:?}`,\n right `{right:?}`")))
    }
}

/// Compares two sorted `Vec<Bytes>`, producing a detailed error on mismatch that includes
/// counts and the items present in one side but not the other.
pub fn assert_equal_bytes_vecs(
    expected: &[Bytes],
    generated: &[Bytes],
    label: &str,
) -> Result<(), Error> {
    if expected == generated {
        return Ok(());
    }

    let expected_set: BTreeSet<&Bytes> = expected.iter().collect();
    let generated_set: BTreeSet<&Bytes> = generated.iter().collect();

    let in_expected_only: Vec<_> = expected_set.difference(&generated_set).collect();
    let in_generated_only: Vec<_> = generated_set.difference(&expected_set).collect();

    let mut msg =
        format!("{label} mismatch — expected {}, generated {}", expected.len(), generated.len());

    if !in_expected_only.is_empty() {
        msg.push_str(&format!(
            "\n  in expected but not generated ({}):\n    {}",
            in_expected_only.len(),
            in_expected_only.iter().map(|b| format!("{b}")).collect::<Vec<_>>().join("\n    ")
        ));
    }
    if !in_generated_only.is_empty() {
        msg.push_str(&format!(
            "\n  in generated but not expected ({}):\n    {}",
            in_generated_only.len(),
            in_generated_only.iter().map(|b| format!("{b}")).collect::<Vec<_>>().join("\n    ")
        ));
    }

    Err(Error::Assertion(msg))
}
