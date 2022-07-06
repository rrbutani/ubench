use ubench::Benchmark;

#[path = "../common/fib.rs"]
mod fib;
pub use fib::*;

#[path = "../common/fib_memoized.rs"]
mod fib_memoized;
pub use fib_memoized::*;

