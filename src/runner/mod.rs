
use super::{Metric, Reporter};

mod support;
pub use support::black_box;
use support::HListIterator;

mod single;
pub use single::build_single as single;

mod suite;
pub use suite::build_suite as suite;

pub struct BenchmarkRunner<L: RunnableBenchmarkList = ()> {
    iterations: usize,
    list: L,
}

impl Default for BenchmarkRunner<()> {
    fn default() -> Self {
        Self {
            iterations: 1,
            list: (),
        }
    }
}

impl BenchmarkRunner {
    pub const fn new() -> BenchmarkRunner<()> {
        BenchmarkRunner {
            iterations: 1,
            list: (),
        }
    }
}

impl<L: RunnableBenchmarkList> BenchmarkRunner<L> {
    pub const fn set_iterations(mut self, it: usize) -> Self {
        self.iterations = it;
        self
    }

    #[allow(clippy::should_implement_trait)]
    pub fn add<X>(self, top_level_benchmark: X) -> BenchmarkRunner<(X, L)>
    where
        (X, L): RunnableBenchmarkList,
    {
        BenchmarkRunner {
            iterations: self.iterations,
            list: (top_level_benchmark, self.list),
        }
    }

    pub fn run<M: Metric, R: Reporter<M::Unit>>(self, metric: &mut M, reporter: &mut R)
    where
        for<'a> HListIterator<'a, dyn RunnableBenchmarkList + 'a>: Clone,
    {
        reporter.top_level_benchmarks(HListIterator(&self.list as _));
        reporter.num_iterations(self.iterations);

        self.list.run(metric, reporter, self.iterations);

        reporter.ended();
    }
}

#[allow(clippy::len_without_is_empty)]
pub trait RunnableBenchmarkList {
    fn run<M: Metric, R: Reporter<M::Unit>>(self, m: &mut M, r: &mut R, iterations: usize)
    where
        Self: Sized;

    /// # TODO: doc comments!
    ///
    /// ## Why Use a Trait Object Here?
    ///
    /// Using recursion and trait objects (to erase the specific types of the
    /// benchmarks in the list) to get the names of the benchmarks in the list
    /// seems bad but I think it's the pragmatic choice here.
    ///
    /// Usually we'd want to expose a static type (i.e. `&'static [&'static
    /// str]`) for stuff like this but, to do that we'd need to use `unsafe` to
    /// transmute the nested type that we _could_ generate using type
    /// shenanigans, `generic-array` style.
    ///
    /// In this case, performance isn't _really_ a concerna and the optimizer
    /// and the LLVM devirtualizer seem to make quick work of this anyways,
    /// successfully boiling away the trait objects:
    /// https://rust.godbolt.org/z/cd89GcfPT
    fn name_and_next<'a>(&'a self) -> Option<(&'static str, &'a (dyn RunnableBenchmarkList + 'a))>;
    fn len(&self) -> usize;
}

impl<'a> Iterator for HListIterator<'a, (dyn RunnableBenchmarkList + 'a)> {
    type Item = &'static str;

    fn next(&mut self) -> Option<Self::Item> {
        let (name, next) = self.0.name_and_next()?;
        self.0 = next;

        Some(name)
    }
}

impl RunnableBenchmarkList for () {
    fn run<M: Metric, R: Reporter<M::Unit>>(self, _m: &mut M, _r: &mut R, _iterations: usize) {}

    fn name_and_next(&self) -> Option<(&'static str, &dyn RunnableBenchmarkList)> {
        None
    }
    fn len(&self) -> usize {
        0
    }
}
