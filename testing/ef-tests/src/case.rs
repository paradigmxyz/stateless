//! Test case definitions

use crate::result::{CaseResult, Error};
use std::{
    fmt::Debug,
    num::NonZeroUsize,
    path::{Path, PathBuf},
    sync::Mutex,
    thread,
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
    ///
    /// Cases run on a fixed set of scoped threads rather than rayon workers. A rayon worker that
    /// waits on nested parallel work steals other cases, so with rayon an unbounded number of
    /// cases, each holding its own temporary database, can be in flight at once.
    pub fn run(self) -> Vec<CaseResult> {
        let threads = thread::available_parallelism().map_or(1, NonZeroUsize::get);
        let cases = Mutex::new(self.test_cases.into_iter());
        thread::scope(|scope| {
            let workers: Vec<_> = (0..threads)
                .map(|_| {
                    scope.spawn(|| {
                        let mut results = Vec::new();
                        loop {
                            let Some((path, case)) = cases.lock().unwrap().next() else {
                                break results;
                            };
                            results.push(CaseResult::new(&path, case.description(), case.run()));
                        }
                    })
                })
                .collect();
            workers.into_iter().flat_map(|worker| worker.join().unwrap()).collect()
        })
    }
}
