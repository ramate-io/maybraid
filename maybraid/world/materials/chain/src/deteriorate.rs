//! CPU deterioration model mirrored by the fragment shader.
//!
//! Weak links (higher [`crate::hash::link_weakness`] of link id + seed) fail
//! first so a single global durability value still reads as uneven weakening
//! along the chain. That hash is the same `hash13(vec3(...))` the shader
//! uses — not a 1D collapse of the three terms.

use crate::hash;
use crate::params::MythicalChainScalars;

/// Which links give way first. Higher is weaker. Stable for a `(link, seed)`.
pub fn link_weakness(link_id: u32, seed: i32) -> f32 {
	hash::link_weakness(link_id as f32, seed as f32)
}

/// Combine global durability, optional localized color, and per-link weakness.
pub fn link_damage(scalars: MythicalChainScalars, weakness: f32, localized: f32) -> f32 {
	let scalars = scalars.clamped();
	let global = scalars.effective_damage();
	let localized = localized.clamp(0.0, 1.0);
	let mixed = global * (1.0 - scalars.localized_blend) + localized * scalars.localized_blend;
	// Weak links reach a given damage sooner; strong links lag.
	let biased = mixed * (0.55 + 0.75 * weakness);
	biased.clamp(0.0, 1.0)
}

/// Shading weights for one link. All channels are `[0, 1]`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LinkLook {
	pub crack: f32,
	pub glow_scale: f32,
	pub ember: f32,
	pub dissolve: f32,
}

/// Normalized atlas-R thresholds for the fissure cross-section.
///
/// Intact hairlines are already wide enough to read at play distance;
/// damage only widens the same bands. Keep in lockstep with
/// `chain_material.wgsl`.
pub fn crack_band_widths(widen: f32) -> (f32, f32, f32) {
	let w = widen.clamp(0.0, 1.0);
	let hair = 0.10 + 0.10 * w;
	let fissure = 0.20 + 0.22 * w;
	let shoulder = fissure * 1.70;
	(hair, fissure, shoulder)
}

impl LinkLook {
	pub fn from_damage(damage: f32, fade: f32) -> Self {
		let damage = damage.clamp(0.0, 1.0);
		let fade = fade.clamp(0.0, 1.0);
		let crack = smoothstep(0.08, 0.72, damage);
		let glow_scale = (1.0 - damage * 0.82) * (1.0 - fade * 0.85);
		let ember = smoothstep(0.42, 0.92, damage) * (1.0 - fade * 0.35);
		let dissolve = (fade * 0.72 + damage * fade * 0.28 + smoothstep(0.78, 1.0, damage) * 0.18)
			.clamp(0.0, 1.0);
		Self { crack, glow_scale, ember, dissolve }
	}
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
	use crate::params::{MythicalChainScalars, MythicalChainState};

	#[test]
	fn weakness_is_stable_and_in_unit_range() -> anyhow::Result<()> {
		let a = link_weakness(3, 11);
		let b = link_weakness(3, 11);
		assert!((a - b).abs() < 1e-6);
		assert!((0.0..=1.0).contains(&a));
		assert_ne!(link_weakness(3, 11), link_weakness(4, 11));
		assert_ne!(link_weakness(3, 11), link_weakness(3, 29));
		assert_eq!(a, crate::hash::link_weakness(3.0, 11.0));
		assert_eq!(a.to_bits(), 0x3ea0_fdc0);
		Ok(())
	}

	#[test]
	fn weak_links_read_more_damaged_at_the_same_durability() -> anyhow::Result<()> {
		let scalars = MythicalChainScalars::for_state(MythicalChainState::Damaged);
		let weak = link_damage(scalars, 0.95, 0.0);
		let strong = link_damage(scalars, 0.05, 0.0);
		assert!(weak > strong);
		assert!(weak <= 1.0);
		assert!(strong >= 0.0);
		Ok(())
	}

	#[test]
	fn localized_blend_opts_into_vertex_damage() -> anyhow::Result<()> {
		let global = MythicalChainScalars::for_state(MythicalChainState::Intact);
		assert!(link_damage(global, 0.5, 1.0) < 0.05);
		let blended = global.with_localized_blend(1.0);
		assert!(link_damage(blended, 0.5, 1.0) > 0.7);
		Ok(())
	}

	#[test]
	fn authored_states_are_visually_distinct() -> anyhow::Result<()> {
		let intact = LinkLook::from_damage(0.0, 0.0);
		let damaged = LinkLook::from_damage(
			MythicalChainScalars::for_state(MythicalChainState::Damaged).effective_damage(),
			MythicalChainState::Damaged.fade(),
		);
		let near = LinkLook::from_damage(
			MythicalChainScalars::for_state(MythicalChainState::NearBreaking).effective_damage(),
			MythicalChainState::NearBreaking.fade(),
		);
		let released = LinkLook::from_damage(
			MythicalChainScalars::for_state(MythicalChainState::Released).effective_damage(),
			MythicalChainState::Released.fade(),
		);
		assert!(intact.glow_scale > damaged.glow_scale);
		assert!(damaged.glow_scale > near.glow_scale);
		assert!(damaged.crack > intact.crack);
		assert!(near.ember > damaged.ember);
		assert!(released.dissolve > near.dissolve);
		assert!(released.glow_scale < 0.2);
		Ok(())
	}

	#[test]
	fn damage_widens_existing_fissure_bands() -> anyhow::Result<()> {
		let (hair_i, fissure_i, shoulder_i) = crack_band_widths(0.0);
		let (hair_d, fissure_d, shoulder_d) = crack_band_widths(0.8);
		assert!(hair_i < fissure_i && fissure_i < shoulder_i);
		assert!(hair_d > hair_i, "hairlines should widen, {hair_i} → {hair_d}");
		assert!(fissure_d > fissure_i);
		assert!(shoulder_d > shoulder_i);
		assert!(
			hair_d < fissure_i || hair_d < fissure_d,
			"the widened hairline must stay inside the fissure"
		);
		Ok(())
	}

	#[test]
	fn derived_damage_matches_one_minus_intact() -> anyhow::Result<()> {
		let scalars = MythicalChainScalars::for_state(MythicalChainState::NearBreaking);
		assert!((scalars.effective_damage() - (1.0 - scalars.intact)).abs() < 1e-5);
		let override_damage = scalars.with_damage(0.3);
		assert!((override_damage.effective_damage() - 0.3).abs() < 1e-5);
		Ok(())
	}
}
