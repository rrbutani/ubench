use std::collections::HashMap;

use crate::Benchmark;

pub fn memoized(n: u64, table: &mut HashMap<u64, u64>) -> u64 {
    match n {
        0 | 1 => n,
        n => {
            if let Some(v) = table.get(&n) {
                *v
            } else {
                let val = memoized(n - 1, table) + memoized(n - 2, table);
                table.insert(n, val);
                val
            }
        }
    }
}
#[derive(Default)]
pub struct Memoized(HashMap<u64, u64>);
impl Benchmark<u64> for Memoized {
    type Res = u64;
    fn setup(&mut self, _inp: &u64) {
        self.0.clear();
    }
    fn run(&mut self, inp: &u64) -> Self::Res {
        memoized(*inp, &mut self.0)
    }
    fn teardown(&mut self, _inp: &u64, _res: Self::Res) {
        // assert_eq!(ANS[*inp as usize], res, "input: {inp}");
    }
}
