#![cfg_attr(docs, feature(doc_cfg))]
#![cfg_attr(not(feature = "std"), no_std)]
#![doc = include_str!("../README.md")]
//!
//! TODO!

mod bench;
pub use bench::Benchmark;

mod runner;
pub use runner::{single, suite, BenchmarkRunner};

mod metrics;
pub use metrics::*;

mod reporters;
pub use reporters::*;



    }

            }
        }
    }
}
