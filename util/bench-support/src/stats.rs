//! Sorted samples, nearest-rank percentiles, and duration summaries.

use std::time::Duration;

use anyhow::Result;

/// Nearest-rank percentile: `round((len - 1) * p)`, half away from zero.
///
/// For `p = 0.50` this is `sorted[len / 2]` for every `len >= 1`, matching both
/// the POI scheme-timing helper and the Gimme `DurationStats` median.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Percentile(pub f64);

impl Percentile {
	pub const MEDIAN: Self = Self(0.50);
	pub const P95: Self = Self(0.95);

	pub fn rank_index(self, len: usize) -> Option<usize> {
		if len == 0 {
			return None;
		}
		let idx = ((len as f64 - 1.0) * self.0).round() as usize;
		Some(idx.min(len - 1))
	}

	pub fn of_durations(self, sorted: &[Duration]) -> Duration {
		match self.rank_index(sorted.len()) {
			Some(idx) => sorted[idx],
			None => Duration::ZERO,
		}
	}

	pub fn of_nanos(self, sorted: &[u128]) -> u128 {
		match self.rank_index(sorted.len()) {
			Some(idx) => sorted[idx],
			None => 0,
		}
	}
}

/// Sorted `Duration` samples used for percentile queries and summaries.
#[derive(Clone, Debug)]
pub struct DurationSamples {
	sorted: Vec<Duration>,
}

impl DurationSamples {
	pub fn from_unsorted(mut samples: Vec<Duration>) -> Self {
		samples.sort_unstable();
		Self { sorted: samples }
	}

	pub fn as_slice(&self) -> &[Duration] {
		&self.sorted
	}

	pub fn percentile(&self, p: f64) -> Duration {
		Percentile(p).of_durations(&self.sorted)
	}

	pub fn min(&self) -> Duration {
		self.sorted.first().copied().unwrap_or(Duration::ZERO)
	}

	pub fn max(&self) -> Duration {
		self.sorted.last().copied().unwrap_or(Duration::ZERO)
	}

	pub fn median(&self) -> Duration {
		Percentile::MEDIAN.of_durations(&self.sorted)
	}

	pub fn total(&self) -> Duration {
		self.sorted.iter().copied().sum()
	}

	pub fn mean(&self) -> Result<Duration> {
		let n = u32::try_from(self.sorted.len())?;
		if n == 0 {
			return Err(anyhow::anyhow!("microbench produced no samples"));
		}
		Ok(self.total() / n)
	}

	pub fn stats(&self) -> Result<DurationStats> {
		if self.sorted.is_empty() {
			return Err(anyhow::anyhow!("microbench produced no samples"));
		}
		Ok(DurationStats {
			min: self.min(),
			median: self.median(),
			mean: self.mean()?,
			max: self.max(),
			p95: self.percentile(Percentile::P95.0),
			total: self.total(),
		})
	}

	/// Empty samples become zeros. POI scheme-timing used this definition.
	pub fn stats_or_zero(&self) -> DurationStats {
		self.stats().unwrap_or(DurationStats::ZERO)
	}
}

/// min / median / mean / max, plus p95 and total for scheme-timing tables.
#[derive(Clone, Copy, Debug)]
pub struct DurationStats {
	pub min: Duration,
	pub median: Duration,
	pub mean: Duration,
	pub max: Duration,
	pub p95: Duration,
	pub total: Duration,
}

impl DurationStats {
	pub const ZERO: Self = Self {
		min: Duration::ZERO,
		median: Duration::ZERO,
		mean: Duration::ZERO,
		max: Duration::ZERO,
		p95: Duration::ZERO,
		total: Duration::ZERO,
	};

	pub fn from_samples(samples: &[Duration]) -> Result<Self> {
		DurationSamples::from_unsorted(samples.to_vec()).stats()
	}

	pub fn per_op(self, operations: u32) -> Result<Self> {
		if operations == 0 {
			return Err(anyhow::anyhow!("cannot normalize by zero operations"));
		}
		Ok(Self {
			min: self.min / operations,
			median: self.median / operations,
			mean: self.mean / operations,
			max: self.max / operations,
			p95: self.p95 / operations,
			total: self.total / operations,
		})
	}

	pub fn format(self) -> String {
		format!("{:?} / {:?} / {:?} / {:?}", self.min, self.median, self.mean, self.max)
	}
}

/// Integer-nanosecond samples, typically `elapsed.as_nanos() / operations` per run.
#[derive(Clone, Debug)]
pub struct NanoSamples {
	sorted: Vec<u128>,
}

impl NanoSamples {
	pub fn from_unsorted(mut samples: Vec<u128>) -> Result<Self> {
		if samples.is_empty() {
			return Err(anyhow::anyhow!("microbench produced no samples"));
		}
		samples.sort_unstable();
		Ok(Self { sorted: samples })
	}

	/// Divide each duration by `operations` in integer nanos, then summarize.
	///
	/// Motion benches historically divided **per run** before min/median/mean.
	/// That is not the same as `DurationStats::per_op` (divide after the
	/// summary) because of integer truncation, so this path keeps the old order.
	pub fn from_durations_per_op(samples: &[Duration], operations: u128) -> Result<Self> {
		if operations == 0 {
			return Err(anyhow::anyhow!("cannot normalize by zero operations"));
		}
		Self::from_unsorted(samples.iter().map(|d| d.as_nanos() / operations).collect())
	}

	pub fn as_slice(&self) -> &[u128] {
		&self.sorted
	}

	pub fn min(&self) -> u128 {
		self.sorted[0]
	}

	pub fn max(&self) -> u128 {
		self.sorted[self.sorted.len() - 1]
	}

	pub fn median(&self) -> u128 {
		Percentile::MEDIAN.of_nanos(&self.sorted)
	}

	pub fn mean(&self) -> u128 {
		self.sorted.iter().sum::<u128>() / self.sorted.len() as u128
	}

	pub fn spread(&self) -> u128 {
		self.max() - self.min()
	}

	pub fn percentile(&self, p: f64) -> u128 {
		Percentile(p).of_nanos(&self.sorted)
	}

	pub fn percentile_millis(&self, p: f64) -> f64 {
		self.percentile(p) as f64 / 1_000_000.0
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn median_rank_matches_len_over_two() -> Result<()> {
		for len in 1..=64 {
			assert_eq!(
				Percentile::MEDIAN.rank_index(len),
				Some(len / 2),
				"p=0.50 rank for len {len}"
			);
		}
		Ok(())
	}

	#[test]
	fn percentile_empty_is_zero() {
		assert_eq!(Percentile::MEDIAN.of_durations(&[]), Duration::ZERO);
		assert_eq!(Percentile::P95.of_nanos(&[]), 0);
		assert_eq!(
			DurationSamples::from_unsorted(Vec::new()).stats_or_zero().median,
			Duration::ZERO
		);
	}

	#[test]
	fn nearest_rank_p95_and_total_match_poi_scheme_timing() -> Result<()> {
		let samples = [
			Duration::from_micros(100),
			Duration::from_micros(200),
			Duration::from_micros(300),
			Duration::from_micros(400),
		];
		let stats = DurationStats::from_samples(&samples)?;
		assert_eq!(stats.min, Duration::from_micros(100));
		assert_eq!(stats.median, Duration::from_micros(300));
		assert_eq!(stats.p95, Duration::from_micros(400));
		assert_eq!(stats.max, Duration::from_micros(400));
		assert_eq!(stats.mean, Duration::from_micros(250));
		assert_eq!(stats.total, Duration::from_micros(1_000));
		assert_eq!(crate::PublishedDuration(stats.median).micros_2(), "300.00");
		assert_eq!(crate::PublishedDuration(stats.total).compact(), "1.00 ms");
		Ok(())
	}

	#[test]
	fn stats_from_samples() -> Result<()> {
		let samples =
			[Duration::from_millis(4), Duration::from_millis(2), Duration::from_millis(6)];
		let stats = DurationStats::from_samples(&samples)?;
		assert_eq!(stats.min, Duration::from_millis(2));
		assert_eq!(stats.median, Duration::from_millis(4));
		assert_eq!(stats.max, Duration::from_millis(6));
		assert_eq!(stats.mean, Duration::from_millis(4));
		assert_eq!(stats.per_op(2)?.mean, Duration::from_millis(2));
		Ok(())
	}

	#[test]
	fn per_run_nanos_division_happens_before_stats() -> Result<()> {
		let samples =
			[Duration::from_nanos(1_005), Duration::from_nanos(2_005), Duration::from_nanos(3_005)];
		let nanos = NanoSamples::from_durations_per_op(&samples, 10)?;
		assert_eq!(nanos.as_slice(), &[100, 200, 300]);
		assert_eq!(nanos.min(), 100);
		assert_eq!(nanos.median(), 200);
		assert_eq!(nanos.mean(), 200);
		assert_eq!(nanos.spread(), 200);
		assert_eq!(nanos.percentile_millis(0.95), 300.0 / 1_000_000.0);
		Ok(())
	}

	#[test]
	fn compact_format_switches_to_seconds() {
		assert_eq!(crate::PublishedDuration(Duration::from_millis(999)).compact(), "999.00 ms");
		assert_eq!(crate::PublishedDuration(Duration::from_secs(2)).compact(), "2.00 s");
	}
}
