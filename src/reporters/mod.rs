
use core::fmt::Debug;

#[allow(unused_variables)]
pub trait Reporter<Unit> {
    fn top_level_benchmarks<I: Iterator<Item = &'static str> + Clone>(&mut self, names: I) {}
    fn num_iterations(&mut self, iterations: usize) {}

    // single benchmarks go in this order:
    // input 1:
    //   + iteration 1
    //   + iteration 2
    //     ...
    // input 2:
    //   + iteration 1
    //   + iteration 2
    //     ...
    //
    fn starting_single_benchmark(
        &mut self,
        name: &'static str,
        inputs_size_hint: (usize, Option<usize>),
    ) {
    }
    fn single_benchmark_run(
        &mut self,
        input_idx: usize,
        input: &dyn Debug,
        iteration_idx: usize,
        measurement: Unit,
    ) {
    }
    fn ending_single_benchmark(&mut self, name: &'static str) {}

    // benchmark suites go in this order:
    // input 1:
    //   - benchmark A
    //     + iteration 1
    //     + iteration 2
    //       ...
    //   - benchmark B
    //     + iteration 1
    //     + iteration 2
    //       ...
    // input 2:
    //  ...
    //
    //
    fn starting_new_benchmark_suite<I: Iterator<Item = &'static str> + Clone>(
        &mut self,
        name: &'static str,
        inputs_size_hint: (usize, Option<usize>),
        benchmark_names: I,
    ) {
    }
    fn suite_benchmark_run(
        &mut self,
        input_idx: usize,
        input: &dyn Debug,
        benchmark_idx: usize,
        benchmark_name: &'static str,
        iteration_idx: usize,
        measurement: Unit,
    ) {
    }
    fn ending_benchmark_suite(&mut self, name: &'static str) {}

    fn ended(&mut self) {}
}

pub struct NoOpReporter;

impl<U> Reporter<U> for NoOpReporter {}

