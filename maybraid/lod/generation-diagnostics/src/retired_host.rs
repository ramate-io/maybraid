//! Slow completions from the existing exclusive retired-host drain.

use std::sync::OnceLock;
use std::time::Duration;

const SLOW_ROOT: Duration = Duration::from_millis(8);
const SAMPLE_LIMIT: u8 = 32;

/// Enables retired-host provenance in a generation-tracing build.
pub fn enabled() -> bool {
	static ENABLED: OnceLock<bool> = OnceLock::new();
	*ENABLED.get_or_init(|| {
		std::env::var_os("MAYBRAID_RETIRED_HOST_PROFILE").is_some_and(|value| value == "1")
	})
}

/// Per-owner cap for slow-root provenance over the lifetime of an ECS app world.
#[derive(Default)]
pub struct RetiredHostSamples {
	emitted: u8,
}

impl RetiredHostSamples {
	pub fn is_full(&self) -> bool {
		self.emitted >= SAMPLE_LIMIT
	}

	fn admit(&mut self, elapsed: Duration) -> Option<u8> {
		if elapsed < SLOW_ROOT || self.is_full() {
			return None;
		}
		self.emitted += 1;
		Some(self.emitted)
	}

	pub fn record(&mut self, sample: RetiredHostCompletion) {
		let Some(sequence) = self.admit(sample.elapsed) else {
			return;
		};
		let net_live_delta = i64::from(sample.live_after) - i64::from(sample.live_before);
		tracing::info!(
			target: "maybraid.hcsg.retired_host",
			"hcsg.retired_host sample={} limit={} threshold_us={} elapsed_us={} root_bits={} scheme={:?} direct_children={} root_disabled={} live_before={} live_after={} net_live_entity_delta={}",
			sequence,
			SAMPLE_LIMIT,
			SLOW_ROOT.as_micros(),
			sample.elapsed.as_micros(),
			sample.root_bits,
			sample.scheme,
			sample.direct_children,
			sample.root_disabled,
			sample.live_before,
			sample.live_after,
			net_live_delta,
		);
	}
}

/// Snapshot across one real `World::despawn` call. Counts are world-wide
/// before/after samples and their delta may include observer work.
pub struct RetiredHostCompletion {
	pub root_bits: u64,
	pub scheme: &'static str,
	pub direct_children: usize,
	pub root_disabled: bool,
	pub live_before: u32,
	pub live_after: u32,
	pub elapsed: Duration,
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn probe_budgets_only_slow_completions() {
		let mut budget = RetiredHostSamples::default();
		assert_eq!(budget.admit(SLOW_ROOT - Duration::from_micros(1)), None);
		for sample in 1..=SAMPLE_LIMIT {
			assert_eq!(budget.admit(SLOW_ROOT), Some(sample));
		}
		assert!(budget.is_full());
		assert_eq!(budget.admit(Duration::from_secs(1)), None);
	}
}
