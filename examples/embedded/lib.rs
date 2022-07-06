#![no_std]

use ubench::Benchmark;

#[path = "../common/fib.rs"]
mod fib;
pub use fib::*;
