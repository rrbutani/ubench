use crate::Benchmark;
use libm::{pow, round, sqrt};

pub fn recursive(n: u64) -> u64 {
    match n {
        0 | 1 => n,
        n => recursive(n - 1) + recursive(n - 2),
    }
}
pub struct Recursive;
impl Benchmark<u64> for Recursive {
    type Res = u64;
    fn run(&mut self, inp: &u64) -> Self::Res {
        recursive(*inp)
    }
    fn teardown(&mut self, inp: &u64, res: Self::Res) {
        assert_eq!(ANS[*inp as usize], res, "input: {inp}");
    }
}

fn closed_form(n: u64) -> u64 {
    //! binet's formula

    // `f64::sqrt` isn't actually in `core`...
    // let root_t = 5f64.sqrt();
    let root_5 = sqrt(5f64);
    let n: i32 = n.try_into().unwrap();

    let lhs = (1. + root_5) / 2.;
    // let lhs = lhs.powi(n);
    let lhs = pow(lhs, n as f64);

    let rhs = (1. - root_5) / 2.;
    // let rhs = rhs.powi(n);
    let rhs = pow(rhs, n as f64);

    let res = (1. / root_5) * (lhs - rhs);
    // res.round() as _
    round(res) as _
}
pub struct ClosedForm;
impl Benchmark<u64> for ClosedForm {
    type Res = u64;
    fn run(&mut self, inp: &u64) -> Self::Res {
        closed_form(*inp)
    }
    fn teardown(&mut self, inp: &u64, res: Self::Res) {
        assert_eq!(ANS[*inp as usize], res, "input: {inp}");
    }
}

fn iterative(n: u64) -> u64 {
    if let 0 | 1 = n {
        return n;
    }

    let mut a = 0;
    let mut b = 1;
    for _ in 0..(n - 1) {
        let next = a + b;
        a = b;
        b = next;
    }

    b
}
pub struct Iterative;
impl Benchmark<u64> for Iterative {
    type Res = u64;
    fn run(&mut self, inp: &u64) -> Self::Res {
        iterative(*inp)
    }
    fn teardown(&mut self, inp: &u64, res: Self::Res) {
        assert_eq!(ANS[*inp as usize], res, "input: {inp}");
    }
}

pub static ANS: [u64; 36] = [
    0, 1, 1, 2, 3, 5, 8, 13, 21, 34, 55, 89, 144, 233, 377, 610, 987, 1597, 2584, 4181, 6765,
    10946, 17711, 28657, 46368, 75025, 121393, 196418, 317811, 514229, 832040, 1346269, 2178309,
    3524578, 5702887, 9227465,
];
