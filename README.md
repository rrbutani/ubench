# `µbench`

> "micro bench", as in: microcontroller

## what

This is a tiny crate that attempts to help you benchmark things running on microcontrollers.

This is **not** a particularly good crate. It:
  - does not attempt to be [statistically rigorous](https://github.com/bheisler/criterion.rs)
  - does not try to mitigate the effects of CPU caches/frequency scaling/OS context switches, etc. on benchmarks
  - does not really provide machinery for host-side processing
  - does not attempt to mimic the output of the [`test::bench` module](https://doc.rust-lang.org/test/bench/index.html)
  - does not make use of the [`defmt` ecosystem](https://github.com/knurling-rs/defmt) [^1] (in order to support boards that do not have `probe-rs` support)

[^1]: Nothing precludes you from writing a [`Reporter`](TODO) that makes use of `defmt` machinery but this crate does not ship with one. The default reporter is naive, uses `core::fmt`, and is space inefficient.

`µbench` is very much intended to be a stopgap; it is my sincere hope that this crate will be [obviated](https://github.com/knurling-rs/defmt/issues/555#issuecomment-1013313850) in the [near future](https://ferrous-systems.com/blog/knurling-summer-of-code/).

However, as of this writing, there seems to be a real dearth of solutions aimed at users who just want to: run some code on their device and get back cycle counts, without needing to spin up a debugger. Hence this crate.

The closest thing out there (that I am aware of) that serves this use case is [`liar`](https://github.com/ranweiler/liar) which is, unfortunately, just a tiny bit too barebones when the `std` feature is disabled.

## how does it work?

(overview of the traits:
  - `Benchmark` which can be: `fn`, closure impling `FnMut`, custom impl with `setup` + `teardown`
    + these take some `Inp` data _by reference_
  - `BenchmarkRunner` lets you actually run benchmarks; two kinds
    + single (constructible with `single`)
      * exactly one `Benchmark` impl
    + suite (constructible with `suite`)
      * zero or more `Benchmark` impls that are all run on the same input data
    + each of these also take some `impl IntoIterator<Item = T>` as an input source where `T: Debug`
      * this can be things like a range (`0..10`), an array (`["hey", "there"]`), an iterator (`(0..10).map(|x| 2u32.pow(x))`), etc.
  - to actually run the benchmarks you need:
    + a `Metric`
      * some way to actually measure the benchmarks; i.e. time, cycle counts
    + a `Reporter`
      * some way to report out the results of the benchmarking
)


## usage

