
use core::fmt::Debug;

use crate::Metric;

#[allow(unused_variables)]
pub trait Reporter<M: Metric> {
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
        measurement: M::Unit,
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
        measurement: M::Unit,
    ) {
    }
    fn ending_benchmark_suite(&mut self, name: &'static str) {}

    fn ended(&mut self) {}
}

/// A placeholder reporter that does nothing.
pub struct NoOpReporter;

impl<M: Metric> Reporter<M> for NoOpReporter {}

macro_rules! feature_gated {
    ($mod_name:ident gated with: $($cfg_expr:tt)*) => {
        #[cfg( $($cfg_expr)* )]
        #[cfg_attr(all(docs, not(doctest)), doc(cfg( $($cfg_expr)* )))]
        mod $mod_name;

        #[cfg( $($cfg_expr)* )]
        pub use $mod_name::*;
    };

    ($mod_name:ident gated on $($features:literal),+) => {
        feature_gated![$mod_name gated with: all( $(feature = $features),+ )];
    };
}

mod io;

mod basic;
pub use basic::*;
// feature_gated![basic gated with: any(feature = "embedded-hal", feature = "std")];

// host side only, has:
//   - pretty curved unicode table things (border colored on type)
//   - single:
//     + listing of results for each input with ± and:
//       * inline stats style |---[    ]----| diagrams for individual results
//     + if inputs are: numbers (u8, u16, u32, u64, u128, usize, i8, i16, i32, i64, i128, isize)
//       and formatted `Unit` is parseable as a number or float
//       * if inputs sequence is order of magnitude: use log scale for x axis
//       * show a dot plot with inputs on the x axis and the metric on the y
//   - suite:
//     + table of results
//       * ± on each entry
//       * bold + colors for best/worst for each input
//     + same as above, if inputs are numbers and formatted `Unit` is a number, then
//       * show a dot plot (log scale if appropriate)
//
// pub struct PrettyPrintReporter<Unit, Out, Divisor = u32>(PhantomData<(Unit, Divisor)>)
// where
//     Unit:
//         Add<Output = Unit> + // We'd like to use `num_traits::CheckedAdd` but this is not impl'd for Duration!
//         Sub<Output = Unit> +
//         Div<Divisor, Output = Unit> +
//         PartialEq +
//         PartialOrd +
//         Display
//         ,
//     Out: std::io::Write,
//     Divisor: TryFrom<usize>,
// ;

// device side, gated on `json`
//
// accepts embedded_hal::serial::Write | std::io::Write (todo: trait to unify these)
pub struct JsonReporter;


// host side, takes an `io::Read`, deserializes it as JSON, feeds it to a
// Reporter (defaults to `PrettyPrintAdapter`)
pub struct JsonToReporterAdapter;

// device side, gated on `embedded-hal`; accepts
// embedded_hal::serial::Write | std::io::Write
// pub struct BasicReporter;
