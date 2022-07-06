
use core::{
    any::Any,
    fmt::Debug,
    marker::PhantomData,
};

use crate::{Benchmark, Metric, Reporter};
use super::{RunnableBenchmarkList, black_box};

pub fn build_single<B: Benchmark<Inp>, Inp: Any + Debug, I: IntoIterator<Item = Inp>>(
    name: &'static str,
    benchmark: B,
    inputs: I,
) -> SingleBenchmark<B, Inp, I> {
    SingleBenchmark {
        name,
        benchmark,
        inputs,
        _p: PhantomData,
    }
}

pub struct SingleBenchmark<B: Benchmark<Inp>, Inp: Any + Debug, I: IntoIterator<Item = Inp>> {
    name: &'static str,
    benchmark: B,
    inputs: I,
    _p: PhantomData<Inp>,
}

impl<B, Inp, I, Rest> RunnableBenchmarkList for (SingleBenchmark<B, Inp, I>, Rest)
where
    B: Benchmark<Inp>,
    Inp: Any + Debug,
    I: IntoIterator<Item = Inp>,
    Rest: RunnableBenchmarkList,
{
    fn run<M: Metric, R: Reporter<M>>(self, m: &mut M, r: &mut R, iterations: usize) {
        let (mut this, rest) = self;

        let inputs = this.inputs.into_iter();
        r.starting_single_benchmark(this.name, inputs.size_hint());

        for (inp_idx, inp) in inputs.enumerate() {
            for it_idx in 0..iterations {
                this.benchmark.setup(&inp);
                let before = m.start();
                #[allow(clippy::unit_arg)]
                black_box(this.benchmark.run(black_box(&inp)));
                let measurement = m.end(before);
                this.benchmark.teardown();

                r.single_benchmark_run(inp_idx, &inp, it_idx, measurement);
            }
        }

        r.ending_single_benchmark(this.name);

        rest.run(m, r, iterations);
    }

    fn name_and_next(&self) -> Option<(&'static str, &dyn RunnableBenchmarkList)> {
        Some((self.0.name, &self.1))
    }

    fn len(&self) -> usize {
        self.1.len() + 1
    }
}
