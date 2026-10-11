//! CPU deterioration model mirrored by the fragment shader.
//!
//! Weak links (higher hash of link id + seed) fail first so a single global
//! durability value still reads as uneven weakening along the chain.

use crate::params::MythicalChainScalars;

/// Which links give way first. Higher is weaker. Stable for a `(link, seed)`.
pub fn link_weakness(link_id: u32, seed: i32) -> f32 {
	hash11(link_id as f32 * 17.13 + seed as f32 * 0.013 + 4.2)
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

impl LinkLook {
	pub fn from_damage(damage: f32, fade: f32) -> Self {
		let damage = damage.clamp(0.0, 1.0);
		let fade = fade.clamp(0.0, 1.0);
		let crack = smoothstep(0.08, 0.72, damage);
		let glow_scale = (1.0 - damage * 0.72) * (1.0 - fade * 0.85);
		let ember = smoothstep(0.42, 0.92, damage) * (1.0 - fade * 0.35);
		let dissolve = (fade * 0.82 + damage * fade * 0.35 + smoothstep(0.78, 1.0, damage) * 0.22)
			.clamp(0.0, 1.0);
		Self { crack, glow_scale, ember, dissolve }
	}
}

fn hash11(p: f32) -> f32 {
	let mut x = p * 0.1031;
	x -= x.floor();
	let k = x + 33.33;
	x = (x + x * k + k) * 0.1031;
	x -= x.floor();
	x
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
	fn derived_damage_matches_one_minus_intact() -> anyhow::Result<()> {
		let scalars = MythicalChainScalars::for_state(MythicalChainState::NearBreaking);
		assert!((scalars.effective_damage() - (1.0 - scalars.intact)).abs() < 1e-5);
		let override_damage = scalars.with_damage(0.3);
		assert!((override_damage.effective_damage() - 0.3).abs() < 1e-5);
		Ok(())
	}
}
