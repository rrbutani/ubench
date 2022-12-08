#![cfg_attr(all(docs, not(doctest)), feature(doc_cfg))]
#![cfg_attr(not(feature = "std"), no_std)]
#![doc = include_str!("../README.md")]
//!
//! TODO!

#![doc(
    html_root_url = "https://docs.rs/ubench/0.0.0-alpha0", // remember to bump!
)]

pub mod bench;
pub use bench::Benchmark;

pub mod runner;
pub use runner::{single, suite, BenchmarkRunner};

pub mod metrics;
pub use metrics::Metric;

pub mod reporters;
pub use reporters::Reporter;

#[cfg(test)]
#[path = "../examples/common/fib.rs"]
mod fib;

#[cfg(test)]
#[path = "../examples/common/fib_memoized.rs"]
mod fib_memoized;

#[cfg(test)]
mod tests {
    extern crate std;

    use super::*;
    use metrics::*;
    use reporters::*;
    use std::{any::Any, fmt::Debug, string::String};

    fn foo(_i: &i32) {}

    struct B;
    impl<T: Debug + Any> Benchmark<T> for B {
        type Res = ();
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
        let mut s = String::new();
        let mut r = BasicReporter::new_with_fmt_write(&mut s);

        // Compile check: make sure single benchmarks and suites work!
        //
        // Also make sure `fn`, closure, and `impl Benchmark` benchmarks work.
        //
        // Multiple input sources too.
        BenchmarkRunner::new()
            .set_iterations(200)
            .add(single("yopp", |_i: &_| {}, [89, 89, 89]))
            .add(single("yo", foo, [89, 89, 89]))
            .add(single("yo", B, std::vec!["erer", "ere", "erer"]))
            .add(single("yo", foo, [89, 89, 89]))
            .add(single("yo", |_i: &_| {}, -23..34))
            .add(single("yo", |_i: &_| {}, (0..1024).step_by(2)))
            .add(single("yo", |_i: &_| {}, (0..10).map(|x| 2u32.pow(x))))
            .add(single("yo", |_i: &_| {}, inputs(2)))
            .add(single("yo", |_i: &_| {}, [89, 89, 89]))
            .add(
                suite("fibonacci comparison", [1, 2, 3])
                    .add("one", foo)
                    .add("two", B)
                    .add("three", |_x: &_| {}),
            )
            .run(&mut m, &mut r);

        std::eprintln!("{}", s);
    }

    #[test]
    #[cfg(feature = "std")]
    fn fibonacci_example() {
        use super::{fib::*, fib_memoized::*};

        let mut out = std::io::stderr();
        let mut m = StdSysTime;
        let mut r = BasicReporter::new_with_io_write(&mut out);

        BenchmarkRunner::new()
            .set_iterations(50)
            .add(
                // suite("fibonacci comparison", (0..7).map(|x| 2u64.pow(x)))
                suite("fibonacci comparison", (0..35).step_by(5))
                    .add("recursive", Recursive)
                    .add("memoized", Memoized::default())
                    .add("iterative", Iterative)
                    .add("closed form", ClosedForm),
            )
            .run(&mut m, &mut r);
    }
}
