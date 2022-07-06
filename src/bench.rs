use core::{any::Any, fmt::Debug};

#[allow(unused_variables)]
pub trait Benchmark<Inp: Any + Debug> {
    type Res;

    /// Called before every call to `run`.
    ///
    /// For stuff you wish to have run once, use a constructor function.
    #[allow(unused_variables)]
    fn setup(&mut self, inp: &Inp) {}

    /// This is what is actually measured.
    fn run(&mut self, inp: &Inp) -> Self::Res;

    /// Called after every call to `run`.
    ///
    /// For stuff you wish to have run once, use `Drop`.
    fn teardown(&mut self, inp: &Inp, res: Self::Res) {}
}

impl<I: Any + Debug, R, F: FnMut(&I) -> R> Benchmark<I> for F {
    type Res = R;

    fn run(&mut self, inp: &I) -> R {
        self(inp)
    }
}
