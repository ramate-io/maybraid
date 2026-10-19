//! Small development-only helpers for Gimme / LOD fill microbenches.
//!
//! Timing, stats, and A/B reporting live in Bevy-free [`bench_support`]. This
//! crate adds the Gimme fill/index harness (host lattice, historical adapters,
//! optional persistent Bevy systems). Scenario setup, destination lifetimes,
//! and correctness checks stay in the calling bench.

mod history;
mod spatial;

pub use bench_support::{
	AbReport, DurationSamples, DurationStats, Microbench, NanoSamples, PassLifetime, Percentile,
	PublishedDuration, Stopwatch, DEFAULT_REPEATS, DEFAULT_WARMUP,
};
pub use history::{ExtendIntersectingFill, PairWalkFill};
pub use spatial::HostGrid;

#[cfg(feature = "bevy")]
mod bevy_sys;

#[cfg(feature = "bevy")]
pub use bevy_sys::{MessageBuffers, PersistentSystem};
