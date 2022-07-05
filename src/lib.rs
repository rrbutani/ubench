#![cfg_attr(docs, feature(doc_cfg))]
#![cfg_attr(not(feature = "std"), no_std)]
#![doc = include_str!("../README.md")]
//!
//! TODO!

use core::{
    any::Any,
    fmt::{self, Debug},
    marker::PhantomData,
    ops::{Add, Div, Sub},
};

pub fn black_box<T>(x: T) -> T {
    // TODO: inline asm method??
    unsafe {
        let ret = core::ptr::read_volatile(&x);
        core::mem::forget(x);
        ret
    }
}

pub trait Benchmark<Inp: Any + Debug> {
    /// Called before every call to `run`.
    ///
    /// For stuff you wish to have run once, use a constructor function.
    #[allow(unused_variables)]
    fn setup(&mut self, inp: &Inp) {}
    /// This is what is actually measured.
    fn run(&mut self, inp: &Inp);
    /// Called after every call to `run`.
    ///
    /// For stuff you wish to have run once, use `Drop`.
    fn teardown(&mut self) {}
}

impl<I: Any + Debug, F: FnMut(&I)> Benchmark<I> for F {
    fn run(&mut self, inp: &I) {
        self(inp)
    }
}

#[derive(Debug, Copy)]
#[doc(hidden)]
pub struct HListIterator<'a, Inner: ?Sized>(&'a Inner);

impl<'a, T: ?Sized> Clone for HListIterator<'a, T> {
    fn clone(&self) -> Self {
        Self(<&T>::clone(&self.0))
    }
}

impl<'a> Iterator for HListIterator<'a, (dyn RunnableBenchmarkList + 'a)> {
    type Item = &'static str;

    fn next(&mut self) -> Option<Self::Item> {
        let (name, next) = self.0.name_and_next()?;
        self.0 = next;

        Some(name)
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
impl RunnableBenchmarkList for () {
    fn run<M: Metric, R: Reporter<M::Unit>>(self, _m: &mut M, _r: &mut R, _iterations: usize) {}

    fn name_and_next(&self) -> Option<(&'static str, &dyn RunnableBenchmarkList)> {
        None
    }
    fn len(&self) -> usize {
        0
    }
}

pub struct SingleBenchmark<B: Benchmark<Inp>, Inp: Any + Debug, I: IntoIterator<Item = Inp>> {
    name: &'static str,
    benchmark: B,
    inputs: I,
    _p: PhantomData<Inp>,
}

pub fn single<B: Benchmark<Inp>, Inp: Any + Debug, I: IntoIterator<Item = Inp>>(
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

impl<B, Inp, I, Rest> RunnableBenchmarkList for (SingleBenchmark<B, Inp, I>, Rest)
where
    B: Benchmark<Inp>,
    Inp: Any + Debug,
    I: IntoIterator<Item = Inp>,
    Rest: RunnableBenchmarkList,
{
    fn run<M: Metric, R: Reporter<M::Unit>>(self, m: &mut M, r: &mut R, iterations: usize) {
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

impl<Inp, I, L, Rest> RunnableBenchmarkList for (Suite<Inp, I, L>, Rest)
where
    Inp: Debug,
    I: IntoIterator<Item = Inp>,
    L: RunnableSuiteBenchmarkList<Inp>,
    Rest: RunnableBenchmarkList,
    for<'a> HListIterator<'a, dyn RunnableSuiteBenchmarkList<Inp> + 'a>: Clone,
{
    fn run<M: Metric, R: Reporter<M::Unit>>(self, m: &mut M, r: &mut R, iterations: usize) {
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

// Like `RunnableBenchmarkList` but specific to suites; we cannot just
// use `RunnableBenchmarkList` because we do not want to allow _recursion_ (i.e.
// we would not know how to handle a benchmark suite being nested within a
// benchmark suite).
#[allow(clippy::len_without_is_empty)]
pub trait RunnableSuiteBenchmarkList<Inp: Debug> {
    fn run<M: Metric, R: Reporter<M::Unit>>(
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
    fn run<M: Metric, R: Reporter<M::Unit>>(
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

impl<I, B, Rest> RunnableSuiteBenchmarkList<I> for (SuiteMember<B, I>, Rest)
where
    I: Any + Debug,
    B: Benchmark<I>,
    Rest: RunnableSuiteBenchmarkList<I>,
{
    fn run<M: Metric, R: Reporter<M::Unit>>(
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
            #[allow(clippy::unit_arg)]
            black_box(this.benchmark.run(black_box(inp)));
            let measurement = m.end(before);
            this.benchmark.teardown();

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

pub struct SuiteMember<B: Benchmark<Inp>, Inp: Any + Debug> {
    name: &'static str,
    benchmark: B,
    _p: PhantomData<Inp>,
}

pub struct Suite<Inp: Debug, I: IntoIterator<Item = Inp>, L: RunnableSuiteBenchmarkList<Inp> = ()> {
    name: &'static str,
    benchmark_list: L,
    inputs: I,
    _p: PhantomData<Inp>,
}

pub fn suite<Inp: Debug, I: IntoIterator<Item = Inp>>(
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

impl Default for BenchmarkRunner<()> {
    fn default() -> Self {
        Self {
            iterations: 1,
            list: (),
        }
    }
}

pub struct BenchmarkRunner<L: RunnableBenchmarkList = ()> {
    iterations: usize,
    list: L,
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

pub trait Metric {
    type Unit: PartialOrd
        + PartialEq
        + Add<Output = Self::Unit>
        + Sub<Output = Self::Unit>
        + Div<Self::Divisor, Output = Self::Unit>
        + Debug;
    type Divisor: TryFrom<usize> /* = Self::Unit */;
    type Start;

    const UNIT_NAME: &'static str = "unknown";

    fn start(&mut self) -> Self::Start;
    fn end(&mut self, start: Self::Start) -> Self::Unit;
    fn print(u: &Self::Unit, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Debug::fmt(u, f)
    }
}

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

pub mod metrics {
    use super::Metric;

    /// A placeholder metric that just returns 1.
    ///
    /// Using this with [`BenchmarkRunner`](crate::BenchmarkRunner) should
    /// yield `1` as the "result" for every benchmark.
    #[derive(Default)]
    pub struct NoOpMetric;

    impl Metric for NoOpMetric {
        type Unit = u32;
        type Start = ();
        type Divisor = u32;

        fn start(&mut self) { }
        fn end(&mut self, (): ()) -> u32 { 1 }
    }

    macro_rules! feature_gated {
        ($mod_name:ident gated on $feature:literal {
            $($i:item)*
        }) => {
            #[cfg(feature = $feature)]
            #[cfg_attr(docs, doc(cfg(feature = $feature)))]
            mod $mod_name {
                use crate::Metric;

                $($i)*
            }

            #[cfg(feature = $feature)]
            #[doc(hidden)]
            pub use $mod_name::*;
        };
    }

    feature_gated! {
        std_sys_time gated on "std" {
            pub struct StdSysTime;

            use std::time::{Duration, Instant};

            impl Metric for StdSysTime {
                type Start = Instant;
                type Unit = Duration;
                type Divisor = u32;

                const UNIT_NAME: &'static str = "time";

                fn start(&mut self) -> Instant {
                    Instant::now()
                }

                fn end(&mut self, s: Instant) -> Duration {
                    s.elapsed()
                }
            }
        }
    }

    feature_gated! {
        cortex_m_metrics gated on "cortex-m" {
            use cortex_m::peripheral::{DWT, DCB};

            /// NOTE: **cannot detect overflows** of the [`DWT` cycle
            /// counter](cortex_m::peripheral::dwt::RegisterBlock::cyccnt).
            ///
            /// This means this metric won't return accurate results for
            /// benchmarks that take longer than `u32::MAX` cycles to run.
            pub struct CortexMCycleCount<'d>(&'d mut DWT);

            impl CortexMCycleCount<'_> {
                pub fn new<'d, 'b>(dwt: &'d mut DWT, dcb: &'b mut DCB) -> Result<CortexMCycleCount<'d>, ()> {
                    // We need a cycle counter to function!
                    if !DWT::has_cycle_counter() {
                        return Err(())
                    }

                    // As per the docs (https://docs.rs/cortex-m/latest/cortex_m/peripheral/struct.DCB.html#method.enable_trace)
                    // enable tracing first so we can use the DWT unit.
                    dcb.enable_trace();

                    // This is needed on some devices.
                    DWT::unlock();

                    // Next, enable the cycle counter:
                    dwt.enable_cycle_counter();

                    // Retain a reference to the DWT unit so we can reset the cycle counter.
                    Ok(CortexMCycleCount(dwt))
                }
            }

            #[derive(Debug)]
            struct Priv;

            #[derive(Debug)]
            pub struct CortexMCycleCountStart(Priv); // Empty type to serve as a witness.
            impl<'d> Metric for CortexMCycleCount<'d> {
                type Start = CortexMCycleCountStart;
                type Unit = u32;
                type Divisor = u32;

                const UNIT_NAME: &'static str = "cycles";

                fn start(&mut self) -> CortexMCycleCountStart {
                    // We zero the counter to start instead of recording a
                    // starting value;
                    //
                    // This lets us simplify the logic in `end` (don't have
                    // account for wrapping) and means that we don't have to
                    // keep a counter value in a register or on the stack while
                    // the benchmark is running.
                    self.0.set_cycle_count(0);

                    CortexMCycleCountStart(Priv)
                }

                fn end(&mut self, _: CortexMCycleCountStart) -> u32 {
                    // Note: we still cannot detect overflows!
                    DWT::cycle_count()
                }
            }
        }
    }

    feature_gated! {
        riscv_metrics gated on "riscv" {
            use riscv::register::cycle;
            /// NOTE: **UNTESTED**.
            ///
            /// We don't check the pre-conditions; as per [cycle](cycle),
            /// this requires other bits to be set first.
            ///
            /// This `Metric` also will not accurately report cycle counts
            /// when the counter overflow _multiple_ times (i.e. when the
            /// cycle count exceeds [`u64::MAX`]).
            pub struct RiscVCycleCount;

            impl Metric for RiscVCycleCount {
                type Start = u64;
                type Unit = u64;
                type Divisor = u64;

                const UNIT_NAME: &'static str = "cycles";

                fn start(&mut self) -> u64 {
                    cycle::read64()
                }

                // TODO: we don't really have a way to guard against overflow
                // here :(
                //
                // If the cycle counter overflowed multiple times we will not
                // know and will not be able to report it.
                fn end(&mut self, s: u64) -> u64 {
                    let end = cycle::read64();
                    if end > s {
                        end - s
                    } else {
                        // TODO: this is probably not entirely right; not all
                        // impls actually have the counter go up to 64-bits, I think?
                        //
                        // https://ibex-core.readthedocs.io/en/latest/03_reference/performance_counters.html
                        u64::MAX - (s - end)
                    }
                }
            }
        }
    }

    feature_gated! {
        embedded_time_metrics gated on "embedded-time" {
            use embedded_time::{Clock, Instant, ConversionError, duration::{Generic, Nanoseconds}};
            use core::fmt;

            pub struct EmbeddedTimeClock<'c, C: Clock>(pub &'c C)
            where
                Generic<C::T>: TryInto<Nanoseconds<u64>, Error = ConversionError>;

            impl<'c, C: Clock> Metric for EmbeddedTimeClock<'c, C>
            where
                Generic<C::T>: TryInto<Nanoseconds<u64>, Error = ConversionError>,
            {
                type Start = Instant<C>;
                type Unit = Nanoseconds<u64>;
                type Divisor = u64;

                const UNIT_NAME: &'static str = "nanoseconds";

                fn start(&mut self) -> Instant<C> {
                    self.0.try_now().unwrap()
                }

                fn end(&mut self, s: Instant<C>) -> Nanoseconds<u64> {
                    let end = self.0.try_now().unwrap();
                    let dur: Generic<C::T> = s.checked_duration_since(&end).unwrap();
                    dur.try_into().unwrap()
                }

                fn print(u: &Self::Unit, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                    fmt::Display::fmt(u, f)
                }
            }
        }
    }
}

// struct JsonReporter<
// struct PrettyPrintReporter<
