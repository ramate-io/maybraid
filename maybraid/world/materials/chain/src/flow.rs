//! Longitudinal energy flow. CPU port of the WGSL `chain_flow` body.
//!
//! `s` is rest-space arc length in metres along the meshed string, not
//! link-local surface UV and not world Y. Phase is driven by simulation
//! time. A signed speed travels along `+s`; a negative speed reverses;
//! zero speed freezes a standing spatial wave.

use std::f32::consts::TAU;

/// Authored traveling-energy controls on the `mythical_chain` recipe.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ChainFlowStyle {
	/// Metres per second along rest `s`. Signed. Zero is a standing wave.
	pub speed_mps: f32,
	/// Crest spacing in metres. Must stay `> 0`.
	pub wavelength_m: f32,
	/// Visible energy between crests.
	pub baseline: f32,
	/// Crest energy. The helper uses `max(peak, baseline)`.
	pub peak: f32,
	/// Additive shell width in metres, independent of tube radius.
	pub halo_width_m: f32,
	/// Halo brightness scale. Tuned once with the body energy.
	pub halo_gain: f32,
}

impl ChainFlowStyle {
	/// ~2.5 s per transit on the 3.1 m playground chain.
	pub const DEFAULT_SPEED_MPS: f32 = 1.25;
	/// Broad crest: about a third of the playground length.
	pub const DEFAULT_WAVELENGTH_M: f32 = 2.2;
	pub const DEFAULT_BASELINE: f32 = 0.36;
	pub const DEFAULT_PEAK: f32 = 1.0;
	/// Tight aura, well under the 0.052 m placeholder tube radius.
	pub const DEFAULT_HALO_WIDTH_M: f32 = 0.0065;
	pub const DEFAULT_HALO_GAIN: f32 = 0.14;

	pub fn clamped(self) -> Self {
		Self {
			speed_mps: self.speed_mps.clamp(-8.0, 8.0),
			wavelength_m: self.wavelength_m.clamp(1e-3, 32.0),
			baseline: self.baseline.clamp(0.0, 1.0),
			peak: self.peak.clamp(0.0, 2.0),
			halo_width_m: self.halo_width_m.clamp(0.0, 0.12),
			halo_gain: self.halo_gain.clamp(0.0, 4.0),
		}
	}

	pub fn with_speed_mps(mut self, speed_mps: f32) -> Self {
		self.speed_mps = speed_mps;
		self
	}

	pub fn reversed(self) -> Self {
		Self { speed_mps: -self.speed_mps, ..self }
	}
}

impl Default for ChainFlowStyle {
	fn default() -> Self {
		Self {
			speed_mps: Self::DEFAULT_SPEED_MPS,
			wavelength_m: Self::DEFAULT_WAVELENGTH_M,
			baseline: Self::DEFAULT_BASELINE,
			peak: Self::DEFAULT_PEAK,
			halo_width_m: Self::DEFAULT_HALO_WIDTH_M,
			halo_gain: Self::DEFAULT_HALO_GAIN,
		}
		.clamped()
	}
}

/// Shared energy used by conduits, broken edges, and the halo.
///
/// `phase = TAU * (s - speed * time) / wavelength`
/// `broad_wave = 0.5 + 0.5 * cos(phase)`
/// `envelope = smoothstep(0.05, 0.95, broad_wave)`
/// `energy = mix(baseline, max(peak, baseline), envelope)`
pub fn chain_flow(s_m: f32, time: f32, flow: ChainFlowStyle) -> f32 {
	let flow = flow.clamped();
	let phase = TAU * (s_m - flow.speed_mps * time) / flow.wavelength_m;
	let broad_wave = 0.5 + 0.5 * phase.cos();
	let envelope = smoothstep(0.05, 0.95, broad_wave);
	let peak = flow.peak.max(flow.baseline);
	flow.baseline * (1.0 - envelope) + peak * envelope
}

/// Smooth hit flash. Rise then decay; never a frame-1 strobe.
pub fn hit_flash(age: f32, strength: f32) -> f32 {
	let rise = smoothstep(0.0, 0.07, age);
	let fall = 1.0 - smoothstep(0.10, 0.55, age);
	(strength.max(0.0) * rise * fall).clamp(0.0, 1.0)
}

fn smoothstep(edge0: f32, edge1: f32, x: f32) -> f32 {
	if (edge1 - edge0).abs() < 1e-6 {
		return if x < edge0 { 0.0 } else { 1.0 };
	}
	let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
	t * t * (3.0 - 2.0 * t)
}

#[cfg(test)]
mod tests {
	use super::*;

	fn crest_s(time: f32, flow: ChainFlowStyle) -> f32 {
		(flow.speed_mps * time).rem_euclid(flow.wavelength_m)
	}

	#[test]
	fn peaks_arrive_in_spatial_order() -> anyhow::Result<()> {
		let flow = ChainFlowStyle::default();
		let early = crest_s(0.20, flow);
		let later = crest_s(0.55, flow);
		assert!(later > early, "positive speed moves the crest toward +s, {early} → {later}");
		assert!(chain_flow(early, 0.20, flow) > 0.9);
		assert!(chain_flow(later, 0.55, flow) > 0.9);
		Ok(())
	}

	#[test]
	fn reverse_speed_arrives_from_the_far_end() -> anyhow::Result<()> {
		let flow = ChainFlowStyle::default().reversed();
		let early = crest_s(0.20, flow);
		let later = crest_s(0.55, flow);
		assert!(later < early, "negative speed moves the crest toward -s, {early} → {later}");
		assert!(chain_flow(early, 0.20, flow) > 0.9);
		assert!(chain_flow(later, 0.55, flow) > 0.9);
		Ok(())
	}

	#[test]
	fn zero_speed_is_a_standing_wave() -> anyhow::Result<()> {
		let flow = ChainFlowStyle::default().with_speed_mps(0.0);
		let a = chain_flow(0.8, 0.0, flow);
		let b = chain_flow(0.8, 1.7, flow);
		assert!((a - b).abs() < 1e-5, "zero speed must not travel, {a} vs {b}");
		let other = chain_flow(1.6, 0.0, flow);
		assert!((a - other).abs() > 0.02, "standing wave still varies along s");
		Ok(())
	}

	#[test]
	fn crest_is_broad_and_baseline_stays_visible() -> anyhow::Result<()> {
		let flow = ChainFlowStyle::default();
		let mut max = 0.0_f32;
		let mut min = 1.0_f32;
		let mut bright = 0u32;
		const STEPS: u32 = 200;
		for i in 0..STEPS {
			let s = i as f32 / STEPS as f32 * flow.wavelength_m;
			let e = chain_flow(s, 0.0, flow);
			max = max.max(e);
			min = min.min(e);
			if e > (flow.baseline + flow.peak) * 0.5 {
				bright += 1;
			}
		}
		assert!(min >= flow.baseline - 1e-4, "baseline must remain, min={min}");
		assert!(max > 0.9, "peak should be reachable, max={max}");
		assert!(
			bright > STEPS / 5,
			"crest should cover a substantial fraction, bright={bright}/{STEPS}"
		);
		Ok(())
	}

	#[test]
	fn hit_flash_rises_and_decays() -> anyhow::Result<()> {
		assert!(hit_flash(0.0, 1.0) < 0.05);
		assert!(hit_flash(0.12, 1.0) > 0.7);
		assert!(hit_flash(0.7, 1.0) < 0.05);
		Ok(())
	}
}
