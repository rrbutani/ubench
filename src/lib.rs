#![cfg_attr(all(docs, not(doctest)), feature(doc_cfg))]
#![cfg_attr(not(feature = "std"), no_std)]
#![doc = include_str!("../README.md")]
//!
//! TODO!

pub mod bench;
pub use bench::Benchmark;

pub mod runner;
pub use runner::{single, suite, BenchmarkRunner};

pub mod metrics;
pub use metrics::Metric;

pub mod reporters;
pub use reporters::Reporter;

#[cfg(test)]
mod tests {
    extern crate std;
    use std::{fmt::Debug, any::Any};
    use super::*;
    use metrics::*;
    use reporters::*;

    fn foo(_i: &i32) {}

    struct B;
    impl<T: Debug + Any> Benchmark<T> for B {
        fn setup(&mut self, _inp: &T) {}
        fn teardown(&mut self) {}
        fn run(&mut self, _inp: &T) {}
    }

    fn inputs(mut n: u8) -> impl IntoIterator<Item = u8> {
        std::iter::from_fn(move || {
            if n == 0 {
                None
            } else {
                n -= 1;
                Some(n)
            }
        })
    }

    #[test]
    fn smoke_test() {
        let mut m = NoOpMetric::default();
        let mut r = NoOpReporter;

        // Compile check: make sure single benchmarks and suites work!
        //
        // Also make sure `fn`, closure, and `impl Benchmark` benchmarks work.
        //
        // Multiple input sources too.
        BenchmarkRunner::new()
            .add(single("yo", foo, [89, 89, 89]))
            .add(single("yo", B, std::vec!["erer", "ere", "erer"]))
            .add(single("yo", foo, [89, 89, 89]))
            .add(single("yo", |_i: &_| {}, [89, 89, 89]))
            .add(single("yo", |_i: &_| {}, -23..34))
            .add(single("yo", |_i: &_| {}, (0..1024).step_by(2)))
            .add(single("yo", |_i: &_| {}, (0..10).map(|x| 2u32.pow(x))))
            .add(single("yo", |_i: &_| {}, inputs(2)))
            .add(single("yo", |_i: &_| {}, [89, 89, 89]))
            .add(
                suite("3", [1, 2, 3])
                    .add("one", foo)
                    .add("two", B)
                    .add("three", |_x: &_| {}),
            )
            .run(&mut m, &mut r);
    }
}
