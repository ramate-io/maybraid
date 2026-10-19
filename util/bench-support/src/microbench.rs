//! Warmup + timed repeats, A/B order, and destination-lifetime labels.

use std::time::{Duration, Instant};

use anyhow::Result;

use crate::stats::DurationStats;

pub const DEFAULT_WARMUP: usize = 1;
pub const DEFAULT_REPEATS: usize = 7;

/// Times a caller-supplied closure. Workload setup stays outside.
pub struct Stopwatch;

impl Stopwatch {
	pub fn time<F: FnOnce()>(run: F) -> Duration {
		let started = Instant::now();
		run();
		started.elapsed()
	}
}

/// How a destination lives across measured passes.
///
/// Callers apply this themselves. Putting it on the report keeps the lifetime
/// in the output instead of burying it in a helper that clears or allocates.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PassLifetime {
	/// Clear in place each pass; keep allocated capacity (cull cache).
	ClearedPerPass,
	/// Allocate a new destination each pass (produce `HashSet` per domain).
	FreshPerPass,
	/// Keep contents across passes. Not a production cull/produce lifetime.
	Retained,
}

impl PassLifetime {
	pub fn label(self) -> &'static str {
		match self {
			Self::ClearedPerPass => "cleared-per-pass (capacity retained)",
			Self::FreshPerPass => "fresh-per-pass",
			Self::Retained => "retained across passes",
		}
	}
}

/// Warmup + timed repeats. One sample is whatever the caller closure does.
#[derive(Clone, Copy, Debug)]
pub struct Microbench {
	pub warmup: usize,
	pub repeats: usize,
}

impl Default for Microbench {
	fn default() -> Self {
		Self { warmup: DEFAULT_WARMUP, repeats: DEFAULT_REPEATS }
	}
}

impl Microbench {
	pub fn repeats(repeats: usize) -> Self {
		Self { warmup: 0, repeats }
	}

	pub fn collect<F>(&self, mut run: F) -> Result<Vec<Duration>>
	where
		F: FnMut() -> Result<()>,
	{
		for _ in 0..self.warmup {
			run()?;
		}
		let mut samples = Vec::with_capacity(self.repeats);
		for _ in 0..self.repeats {
			samples.push(Self::time(&mut run)?);
		}
		Ok(samples)
	}

	pub fn collect_ok<F>(&self, mut run: F) -> Vec<Duration>
	where
		F: FnMut(),
	{
		for _ in 0..self.warmup {
			run();
		}
		let mut samples = Vec::with_capacity(self.repeats);
		for _ in 0..self.repeats {
			samples.push(Stopwatch::time(&mut run));
		}
		samples
	}

	pub fn collect_per_op_nanos<F>(
		&self,
		operations: u128,
		run: F,
	) -> Result<crate::stats::NanoSamples>
	where
		F: FnMut(),
	{
		crate::stats::NanoSamples::from_durations_per_op(&self.collect_ok(run), operations)
	}

	/// Time A and B in alternating order so sample `i` does not always run A first.
	pub fn collect_ab_alternating<A, B>(
		&self,
		mut original: A,
		mut revised: B,
	) -> Result<(Vec<Duration>, Vec<Duration>)>
	where
		A: FnMut() -> Result<()>,
		B: FnMut() -> Result<()>,
	{
		for _ in 0..self.warmup {
			original()?;
			revised()?;
		}
		let mut original_samples = Vec::with_capacity(self.repeats);
		let mut revised_samples = Vec::with_capacity(self.repeats);
		for i in 0..self.repeats {
			if i % 2 == 0 {
				original_samples.push(Self::time(&mut original)?);
				revised_samples.push(Self::time(&mut revised)?);
			} else {
				revised_samples.push(Self::time(&mut revised)?);
				original_samples.push(Self::time(&mut original)?);
			}
		}
		Ok((original_samples, revised_samples))
	}

	fn time<F: FnMut() -> Result<()>>(run: &mut F) -> Result<Duration> {
		let started = Instant::now();
		run()?;
		Ok(started.elapsed())
	}
}

/// A/B reporter: window totals, per-operation times, and mean ratio.
pub struct AbReport<'a> {
	pub label: &'a str,
	pub original_name: &'a str,
	pub revised_name: &'a str,
	pub original: &'a [Duration],
	pub revised: &'a [Duration],
	pub operations: u32,
	pub operation_name: &'a str,
	pub lifetime: PassLifetime,
	pub notes: &'a str,
}

impl AbReport<'_> {
	pub fn write_to_stderr(&self) -> Result<()> {
		let original = DurationStats::from_samples(self.original)?;
		let revised = DurationStats::from_samples(self.revised)?;
		let original_per = original.per_op(self.operations)?;
		let revised_per = revised.per_op(self.operations)?;
		let ratio = if revised.mean.is_zero() {
			f64::INFINITY
		} else {
			original.mean.as_secs_f64() / revised.mean.as_secs_f64()
		};
		eprintln!(
			"{label}: {notes}; lifetime {lifetime}; \
			 {original_name} min/med/mean/max {original}; \
			 {revised_name} {revised}; \
			 per-{operation} original {original_per} revised {revised_per}; \
			 mean ratio {ratio:.2}×",
			label = self.label,
			notes = self.notes,
			lifetime = self.lifetime.label(),
			original_name = self.original_name,
			original = original.format(),
			revised_name = self.revised_name,
			revised = revised.format(),
			operation = self.operation_name,
			original_per = original_per.format(),
			revised_per = revised_per.format(),
			ratio = ratio,
		);
		Ok(())
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn alternating_order_runs_both_sides() -> Result<()> {
		let mut original_n = 0usize;
		let mut revised_n = 0usize;
		let (original, revised) = Microbench { warmup: 1, repeats: 4 }.collect_ab_alternating(
			|| {
				original_n += 1;
				Ok(())
			},
			|| {
				revised_n += 1;
				Ok(())
			},
		)?;
		assert_eq!(original.len(), 4);
		assert_eq!(revised.len(), 4);
		assert_eq!(original_n, 5);
		assert_eq!(revised_n, 5);
		Ok(())
	}

	#[test]
	fn collect_ok_skips_warmup_samples() {
		let mut n = 0usize;
		let samples = Microbench { warmup: 2, repeats: 3 }.collect_ok(|| n += 1);
		assert_eq!(n, 5);
		assert_eq!(samples.len(), 3);
	}
}
