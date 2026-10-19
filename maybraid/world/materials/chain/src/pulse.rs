//! Chain-wide glow pulse. CPU port of the WGSL `chain_pulse` body.
//!
//! Dominant phase is shared by every link. A small per-link term sits
//! underneath so the chain reads as one object, not a row of independent
//! blinks. Trough is deep; the crest is short.

/// Baseline glow between crests (`0.16`).
pub const PULSE_TROUGH: f32 = 0.16;
/// Peak of the shared crest (`1.0`).
pub const PULSE_CREST: f32 = 1.0;
/// Crest center in the unit period.
pub const PULSE_CREST_CENTER: f32 = 0.86;
/// Half-width of the crest in the unit period.
pub const PULSE_CREST_HALF: f32 = 0.055;
/// Amplitude of the per-link overlay.
pub const PULSE_LINK_VARIATION: f32 = 0.05;

/// Shared pulse in `[0, 1]`. `link_id` only adds a small offset.
pub fn chain_pulse(time: f32, pulse_rate: f32, link_id: f32) -> f32 {
	let rate = pulse_rate.max(0.05);
	let phase = (time * rate).rem_euclid(1.0);
	let t = ((phase - PULSE_CREST_CENTER).abs() / PULSE_CREST_HALF).min(1.0);
	let crest = (1.0 - t) * (1.0 - t);
	let shared = PULSE_TROUGH + (PULSE_CREST - PULSE_TROUGH) * crest;
	let link = PULSE_LINK_VARIATION * (link_id * 2.1 + time * rate * 0.35).sin();
	(shared + link).clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn pulse_has_a_deep_trough_and_short_crest() -> anyhow::Result<()> {
		let rate = 1.4;
		let mut min = 1.0_f32;
		let mut max = 0.0_f32;
		let mut bright = 0u32;
		const STEPS: u32 = 200;
		for i in 0..STEPS {
			let time = i as f32 / (STEPS as f32 * rate);
			let p = chain_pulse(time, rate, 3.0);
			min = min.min(p);
			max = max.max(p);
			if p > 0.7 {
				bright += 1;
			}
		}
		assert!(min < 0.25, "trough should be pronounced, min={min}");
		assert!(max > 0.9, "crest should reach near 1, max={max}");
		assert!(bright < STEPS / 4, "crest should be short, bright_samples={bright}/{STEPS}");
		Ok(())
	}

	#[test]
	fn links_stay_in_sync() -> anyhow::Result<()> {
		let rate = 1.4;
		let mut max_delta = 0.0_f32;
		for i in 0..80 {
			let time = i as f32 * 0.05;
			let a = chain_pulse(time, rate, 0.0);
			let b = chain_pulse(time, rate, 7.0);
			max_delta = max_delta.max((a - b).abs());
		}
		assert!(max_delta < 0.12, "per-link variation should stay small, max_delta={max_delta}");
		Ok(())
	}
}
