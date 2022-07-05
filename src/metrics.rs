
use core::{
    fmt::{self, Debug},
    ops::{Add, Div, Sub},
};

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
