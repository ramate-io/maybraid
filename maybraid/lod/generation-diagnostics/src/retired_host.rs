//! Slow completions from the existing exclusive retired-host drain.

use std::sync::OnceLock;
use std::time::Duration;

const SLOW_ROOT: Duration = Duration::from_millis(8);
const SAMPLE_LIMIT: u8 = 32;
const TRANSITION_LIMIT: u8 = 3;
const TRANSITIONS: [&str; 5] = ["version", "epoch", "bounds", "mob_boundary", "mob_reset"];

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
	transitions: [u8; TRANSITIONS.len()],
}

impl RetiredHostSamples {
	pub fn wants(&self, reason: &str) -> bool {
		self.emitted < SAMPLE_LIMIT
			|| self
				.transition_slot(reason)
				.is_some_and(|slot| self.transitions[slot] < TRANSITION_LIMIT)
	}

	fn transition_slot(&self, reason: &str) -> Option<usize> {
		TRANSITIONS.iter().position(|known| *known == reason)
	}

	fn admit(&mut self, reason: &str, elapsed: Duration) -> Option<(&'static str, u8, u8)> {
		if elapsed >= SLOW_ROOT && self.emitted < SAMPLE_LIMIT {
			self.emitted += 1;
			return Some(("slow", self.emitted, SAMPLE_LIMIT));
		}
		let slot = self.transition_slot(reason)?;
		let count = &mut self.transitions[slot];
		if *count >= TRANSITION_LIMIT {
			return None;
		}
		*count += 1;
		Some(("transition", *count, TRANSITION_LIMIT))
	}

	pub fn record(&mut self, sample: RetiredHostCompletion) {
		let Some((kind, sequence, limit)) = self.admit(sample.reason, sample.elapsed) else {
			return;
		};
		let net_live_delta = i64::from(sample.live_after) - i64::from(sample.live_before);
		tracing::info!(
			target: "maybraid.hcsg.retired_host",
			"hcsg.retired_host kind={} sample={} limit={} slow_threshold_us={} elapsed_us={} root_bits={} scheme={:?} reason={} last_focus_xz_distance_m={:?} successor_bits_and_streamed={:?} queued_us={:?} direct_children={} root_disabled={} live_before={} live_after={} net_live_entity_delta={}",
			kind,
			sequence,
			limit,
			SLOW_ROOT.as_micros(),
			sample.elapsed.as_micros(),
			sample.root_bits,
			sample.scheme,
			sample.reason,
			sample.last_focus_xz_distance_m,
			sample.successor,
			sample.queued_for.map(|duration| duration.as_micros()),
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
	pub reason: &'static str,
	pub last_focus_xz_distance_m: Option<f32>,
	pub successor: Option<(u64, Option<bool>)>,
	pub queued_for: Option<Duration>,
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
	fn probe_budgets_slow_completions_and_transitions_independently() {
		let mut budget = RetiredHostSamples::default();
		let fast = SLOW_ROOT - Duration::from_micros(1);
		assert_eq!(budget.admit("unknown", fast), None);
		for sample in 1..=SAMPLE_LIMIT {
			assert_eq!(budget.admit("unknown", SLOW_ROOT), Some(("slow", sample, SAMPLE_LIMIT)));
		}
		assert!(!budget.wants("unknown"));
		assert_eq!(budget.admit("unknown", Duration::from_secs(1)), None);
		for sample in 1..=TRANSITION_LIMIT {
			assert_eq!(
				budget.admit("version", fast),
				Some(("transition", sample, TRANSITION_LIMIT))
			);
		}
		assert!(!budget.wants("version"));
		assert_eq!(budget.admit("version", fast), None);
		assert_eq!(budget.admit("mob_boundary", fast), Some(("transition", 1, TRANSITION_LIMIT)));
	}
}
