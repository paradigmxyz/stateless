//! Test case definitions

use crate::result::{CaseResult, Error};
use rayon::prelude::*;
use std::{
    fmt::Debug,
    path::{Path, PathBuf},
};

/// A single test case, capable of loading a JSON description of itself and running it.
///
/// See <https://ethereum-tests.readthedocs.io/> for test specs.
pub trait Case: Debug + Send + Sync + Sized + 'static {
    /// A description of the test.
    fn description(&self) -> String {
        "no description".to_string()
    }

    /// Load the test from the given file path.
    ///
    /// The file can be assumed to be a valid EF test case as described on <https://ethereum-tests.readthedocs.io/>.
    fn load(path: &Path) -> Result<Self, Error>;

    /// Return the names of all individual test cases contained in this case.
    fn test_names(&self) -> Vec<&str>;

    /// Keep only the test cases whose name contains the given substring.
    fn filter_by_name(&mut self, filter: &str);

    /// Run the test.
    fn run(self) -> Result<(), Error>;
}

/// A container for multiple test cases.
#[derive(Debug)]
pub struct Cases<T> {
    /// The contained test cases and the path to each test.
    pub test_cases: Vec<(PathBuf, T)>,
}

impl<T: Case> Cases<T> {
    /// Run the contained test cases.
    pub fn run(self) -> Vec<CaseResult> {
        self.test_cases
            .into_par_iter()
            .map(|(path, case)| CaseResult::new(&path, case.description(), case.run()))
            .collect()
    }
}
