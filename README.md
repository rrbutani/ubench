# `µbench`

> "micro bench", as in: microcontroller

## what

This is a tiny crate that attempts to help you benchmark things running on microcontrollers.

This is **not** a particularly good crate. It:
  - does not attempt to be [statistically rigorous](https://github.com/bheisler/criterion.rs)
  - does not provide mechanisms for host-side processing
  - does not attempt to mimic the output of the [`test::bench` module](https://doc.rust-lang.org/test/bench/index.html)
  - does not make use of the [`defmt` ecosystem](https://github.com/knurling-rs/defmt) [^1] (in order to support boards that do not have `probe-rs` support)

[^1]: Nothing precludes you from writing a [`Reporter`](TODO) that makes use of `defmt` machinery but this crate does not ship with one. The default reporter is naive, uses `core::fmt`, and is space inefficient.

`µbench` is very much intended to be a stopgap; it is my sincere hope that this crate will be [obviated](https://github.com/knurling-rs/defmt/issues/555#issuecomment-1013313850) in the [near future](https://ferrous-systems.com/blog/knurling-summer-of-code/).

However, as of this writing, there seems to be a real dearth of solutions aimed at users who just want to: run some code on their device and get back cycle counts, without needing to spin up a debugger. Hence this crate.

The closest thing out there (that I am aware of) that serves this use case is [`liar`](https://github.com/ranweiler/liar) which is, unfortunately, just a tiny bit too barebones when the `std` feature is disabled.

## how does it work?



## usage

