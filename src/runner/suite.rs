
use core::{
    any::Any,
    fmt::Debug,
    marker::PhantomData,
};

use crate::{Benchmark, Metric, Reporter};
use super::{HListIterator, RunnableBenchmarkList, black_box};


pub fn build_suite<Inp: Debug, I: IntoIterator<Item = Inp>>(
    name: &'static str,
    inputs: I,
) -> Suite<Inp, I, ()> {
    Suite {
        name,
        benchmark_list: (),
        inputs,
        _p: PhantomData,
    }
}

pub struct Suite<Inp: Debug, I: IntoIterator<Item = Inp>, L: RunnableSuiteBenchmarkList<Inp> = ()> {
    name: &'static str,
    benchmark_list: L,
    inputs: I,
    _p: PhantomData<Inp>,
}

impl<Inp: Any + Debug, I: IntoIterator<Item = Inp>, L: RunnableSuiteBenchmarkList<Inp>> Suite<Inp, I, L> {
    pub fn add<B: Benchmark<Inp>>(
        self,
        name: &'static str,
        benchmark: B,
    ) -> Suite<Inp, I, (SuiteMember<B, Inp>, L)> {
        let x = SuiteMember {
            name,
            benchmark,
            _p: PhantomData,
        };

        Suite {
            name: self.name,
            benchmark_list: (x, self.benchmark_list),
            inputs: self.inputs,
            _p: PhantomData,
        }
    }
}

#[doc(hidden)]
pub struct SuiteMember<B: Benchmark<Inp>, Inp: Any + Debug> {
    name: &'static str,
    benchmark: B,
    _p: PhantomData<Inp>,
}

// Like `RunnableBenchmarkList` but specific to suites; we cannot just
// use `RunnableBenchmarkList` because we do not want to allow _recursion_ (i.e.
// we would not know how to handle a benchmark suite being nested within a
// benchmark suite).
#[allow(clippy::len_without_is_empty)]
pub trait RunnableSuiteBenchmarkList<Inp: Debug> {
    fn run<M: Metric, R: Reporter<M>>(
        &mut self,
        m: &mut M,
        r: &mut R,
        iterations: usize,
        inp_idx: usize,
        inp: &Inp,
        benchmark_idx: usize,
    ) where
        Self: Sized;

    fn name_and_next(&self) -> Option<(&'static str, &dyn RunnableSuiteBenchmarkList<Inp>)>;

    fn len(&self) -> usize;
}

impl<I: Debug> RunnableSuiteBenchmarkList<I> for () {
    fn run<M: Metric, R: Reporter<M>>(
        &mut self,
        _m: &mut M,
        _r: &mut R,
        _iterations: usize,
        _inp_idx: usize,
        _inp: &I,
        _benchmark_idx: usize,
    ) {
    }
    fn name_and_next(&self) -> Option<(&'static str, &dyn RunnableSuiteBenchmarkList<I>)> {
        None
    }
    fn len(&self) -> usize {
        0
    }
}

impl<'a, I: Debug> Iterator for HListIterator<'a, (dyn RunnableSuiteBenchmarkList<I> + 'a)> {
    type Item = &'static str;

    fn next(&mut self) -> Option<Self::Item> {
        let (name, next) = self.0.name_and_next()?;
        self.0 = next;

        Some(name)
    }
}

impl<I, B, Rest> RunnableSuiteBenchmarkList<I> for (SuiteMember<B, I>, Rest)
where
    I: Any + Debug,
    B: Benchmark<I>,
    Rest: RunnableSuiteBenchmarkList<I>,
{
    fn run<M: Metric, R: Reporter<M>>(
        &mut self,
        m: &mut M,
        r: &mut R,
        iterations: usize,
        inp_idx: usize,
        inp: &I,
        benchmark_idx: usize,
    ) {
        let (ref mut this, rest) = self;

        for it_idx in 0..iterations {
            this.benchmark.setup(inp);
            let before = m.start();
            let res = black_box(this.benchmark.run(black_box(inp)));
            let measurement = m.end(before);
            this.benchmark.teardown(inp, res);

            r.suite_benchmark_run(inp_idx, inp, benchmark_idx, this.name, it_idx, measurement);
        }

        rest.run(m, r, iterations, inp_idx, inp, benchmark_idx + 1);
    }

    fn name_and_next(&self) -> Option<(&'static str, &dyn RunnableSuiteBenchmarkList<I>)> {
        Some((self.0.name, &self.1))
    }

    fn len(&self) -> usize {
        self.1.len() + 1
    }
}

impl<Inp, I, L, Rest> RunnableBenchmarkList for (Suite<Inp, I, L>, Rest)
where
    Inp: Debug,
    I: IntoIterator<Item = Inp>,
    L: RunnableSuiteBenchmarkList<Inp>,
    Rest: RunnableBenchmarkList,
    for<'a> HListIterator<'a, dyn RunnableSuiteBenchmarkList<Inp> + 'a>: Clone,
{
    fn run<M: Metric, R: Reporter<M>>(self, m: &mut M, r: &mut R, iterations: usize) {
        let (mut this, rest) = self;

        let inputs = this.inputs.into_iter();
        r.starting_new_benchmark_suite(
            this.name,
            inputs.size_hint(),
            HListIterator(&this.benchmark_list as _),
        );

        for (inp_idx, inp) in inputs.enumerate() {
            this.benchmark_list.run(m, r, iterations, inp_idx, &inp, 0);
        }

        r.ending_benchmark_suite(this.name);

        rest.run(m, r, iterations);
    }

    fn name_and_next(&self) -> Option<(&'static str, &dyn RunnableBenchmarkList)> {
        Some((self.0.name, &self.1))
    }

    fn len(&self) -> usize {
        self.1.len() + 1
    }
}
