//! Bevy-free timing, statistics, and A/B reporting for development benches.
//!
//! Scenario setup, destination lifetimes, and correctness checks stay in the
//! calling bench. This crate only times a caller-supplied closure and summarizes
//! the samples.

mod format;
mod microbench;
mod stats;

pub use format::PublishedDuration;
pub use microbench::{
	AbReport, Microbench, PassLifetime, Stopwatch, DEFAULT_REPEATS, DEFAULT_WARMUP,
};
pub use stats::{DurationSamples, DurationStats, NanoSamples, Percentile};
