
use core::{
    any::Any,
    fmt::Debug,
};

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
