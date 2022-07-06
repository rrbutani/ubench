
use ubench_host_example::*;
use ubench::{*, metrics::*, reporters::*};

fn main() {
    let mut out = std::io::stderr();
    let mut m = StdSysTime;
    let mut r = BasicReporter::new_with_io_write(&mut out);


    BenchmarkRunner::new()
        .set_iterations(20)
        .add(
            suite("fibonacci comparison", (0..36).step_by(5))
            .add("recursive", Recursive)
            .add("memoized", Memoized::default())
            .add("iterative", Iterative)
            .add("closed form", ClosedForm)
        )
        .run(&mut m, &mut r);
}
