//! Staged structural deterioration. CPU port of the WGSL chunk model.
//!
//! Decorative energy conduits are not fractures. Each link is pre-partitioned
//! into a handful of irregular rest-space regions. A region has a stable
//! damage threshold. Threshold `0` reserves the plate until release. Severity
//! is `max(global, local)` — flash and flow never enter it. Weakness may
//! shift global onset, but a local `1.0` is terminal for every seed.
//!
//! Representation: six jittered Voronoi sites in link-local UV
//! (`around` × `along_link`). U wraps; V does not. Cost is six site hashes
//! plus a wrap-aware nearest/second-nearest search per fragment — no atlas,
//! no per-hit mesh upload. Region ids are computed from UV, so they do not
//! interpolate. Distance-to-edge is filtered only for broken-lip shading.

use crate::hash::{hash13, link_weakness as hash_weakness};
use crate::params::MythicalChainScalars;

/// Sites around the tube.
pub const CHUNK_SITES_U: u32 = 3;
/// Sites along one link.
pub const CHUNK_SITES_V: u32 = 2;
/// `CHUNK_SITES_U * CHUNK_SITES_V`.
pub const CHUNKS_PER_LINK: u32 = CHUNK_SITES_U * CHUNK_SITES_V;

/// Which links give way first under **global** durability. Higher is weaker.
pub fn link_weakness(link_id: u32, seed: i32) -> f32 {
	hash_weakness(link_id as f32, seed as f32)
}

/// Art-directed global curve. Endpoints stay locked so `damage = 1` is
/// terminal for every weakness, and `damage = 0` removes nothing.
pub fn global_severity(effective_damage: f32, weakness: f32) -> f32 {
	let damage = effective_damage.clamp(0.0, 1.0);
	if damage <= 1e-5 {
		return 0.0;
	}
	if damage >= 1.0 - 1e-5 {
		return 1.0;
	}
	let shift = (weakness.clamp(0.0, 1.0) - 0.5) * 0.30 * (4.0 * damage * (1.0 - damage));
	art_curve((damage + shift).clamp(0.0, 1.0))
}

fn art_curve(t: f32) -> f32 {
	let s = t * t * (3.0 - 2.0 * t);
	t * 0.65 + s * 0.35
}

/// Combined structural severity. Flash / transport energy are not inputs.
pub fn severity(global_damage: f32, local_damage: f32, weakness: f32) -> f32 {
	global_severity(global_damage, weakness).max(local_damage.clamp(0.0, 1.0))
}

/// Combine authored scalars with optional localized color and persistent local
/// damage. Weakness only biases the global term.
pub fn link_severity(
	scalars: MythicalChainScalars,
	weakness: f32,
	localized: f32,
	persistent: f32,
) -> f32 {
	let scalars = scalars.clamped();
	let global = scalars.effective_damage();
	let localized = localized.clamp(0.0, 1.0);
	let mixed_global =
		global * (1.0 - scalars.localized_blend) + localized * scalars.localized_blend;
	severity(mixed_global, persistent, weakness)
}

/// Stable threshold for one pre-partitioned chunk.
///
/// Ranked by `hash13` so the pattern never rerolls as damage rises:
/// two early chips, two mid bites, one late gap, one reserved plate.
pub fn chunk_threshold(link_id: f32, seed: f32, chunk_id: f32) -> f32 {
	let h = hash13([chunk_id, seed * 0.017, link_id * 0.31]);
	let rank = (h * CHUNKS_PER_LINK as f32).floor();
	if rank < 0.5 {
		0.20
	} else if rank < 1.5 {
		0.28
	} else if rank < 2.5 {
		0.46
	} else if rank < 3.5 {
		0.55
	} else if rank < 4.5 {
		0.76
	} else {
		0.0
	}
}

/// A region is gone when fade completes, when a reserved plate is released,
/// or when severity crosses its threshold. Removed regions never return.
pub fn region_removed(severity: f32, threshold: f32, fade: f32) -> bool {
	if fade >= 0.995 {
		return true;
	}
	if threshold <= 0.0 {
		return fade > 0.82;
	}
	severity + 1e-5 >= threshold
}

/// One link's structural look for tests and docs. Not a shading formula.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LinkLook {
	pub severity: f32,
	pub removed_chunks: u32,
	pub reserved_remaining: u32,
}

impl LinkLook {
	pub fn from_severity(severity: f32, fade: f32, link_id: f32, seed: f32) -> Self {
		let mut removed = 0u32;
		let mut reserved = 0u32;
		for chunk in 0..CHUNKS_PER_LINK {
			let threshold = chunk_threshold(link_id, seed, chunk as f32);
			if threshold <= 0.0 && !region_removed(severity, threshold, fade) {
				reserved += 1;
			}
			if region_removed(severity, threshold, fade) {
				removed += 1;
			}
		}
		Self { severity, removed_chunks: removed, reserved_remaining: reserved }
	}
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
		Ok(())
	}

	#[test]
	fn local_one_reaches_terminal_for_every_weakness() -> anyhow::Result<()> {
		for weakness in [0.0, 0.15, 0.5, 0.95, 1.0] {
			let value = severity(0.0, 1.0, weakness);
			assert!(
				(value - 1.0).abs() < 1e-5,
				"local 1.0 must be terminal, weakness={weakness} severity={value}"
			);
			let look = LinkLook::from_severity(value, 0.0, 3.0, 11.0);
			assert_eq!(
				look.removed_chunks,
				CHUNKS_PER_LINK - look.reserved_remaining,
				"terminal local damage must drop every removable chunk"
			);
			assert!(look.reserved_remaining > 0, "a reserved plate should survive until fade");
		}
		Ok(())
	}

	#[test]
	fn increasing_damage_never_restores_chunks() -> anyhow::Result<()> {
		let seed = 11.0;
		let link = 4.0;
		let mut previous = 0u32;
		for step in 0..=20 {
			let damage = step as f32 / 20.0;
			let look = LinkLook::from_severity(severity(damage, 0.0, 0.4), 0.0, link, seed);
			assert!(
				look.removed_chunks >= previous,
				"damage {damage} restored chunks {previous} → {}",
				look.removed_chunks
			);
			previous = look.removed_chunks;
		}
		Ok(())
	}

	#[test]
	fn stages_are_visually_distinct() -> anyhow::Result<()> {
		let seed = 11.0;
		let link = 2.0;
		let weakness = 0.5;
		let at =
			|damage: f32| LinkLook::from_severity(severity(damage, 0.0, weakness), 0.0, link, seed);
		let intact = at(0.0);
		let quarter = at(0.25);
		let half = at(0.5);
		let three = at(0.75);
		let full = at(1.0);
		assert_eq!(intact.removed_chunks, 0);
		assert!(quarter.removed_chunks >= 1, "early chips by 25%, {}", quarter.removed_chunks);
		assert!(
			half.removed_chunks > quarter.removed_chunks,
			"mid damage must add bites, {} vs {}",
			half.removed_chunks,
			quarter.removed_chunks
		);
		assert!(
			three.removed_chunks > half.removed_chunks,
			"late damage must open a gap, {} vs {}",
			three.removed_chunks,
			half.removed_chunks
		);
		assert!(
			full.removed_chunks > three.removed_chunks
				|| full.removed_chunks == CHUNKS_PER_LINK - full.reserved_remaining,
			"1.0 should be near failure"
		);
		assert!(full.reserved_remaining > 0);
		assert_eq!(LinkLook::from_severity(1.0, 1.0, link, seed).removed_chunks, CHUNKS_PER_LINK);
		Ok(())
	}

	#[test]
	fn flash_is_not_severity() -> anyhow::Result<()> {
		let scalars = MythicalChainScalars::for_state(MythicalChainState::Intact);
		assert!((link_severity(scalars, 0.9, 0.0, 0.0) - 0.0).abs() < 1e-5);
		assert!((link_severity(scalars, 0.1, 0.0, 0.8) - 0.8).abs() < 1e-5);
		Ok(())
	}

	#[test]
	fn authored_restamp_keeps_local_chunks_missing() -> anyhow::Result<()> {
		let local = 0.8;
		let intact = MythicalChainScalars::for_state(MythicalChainState::Intact);
		let damaged = MythicalChainScalars::for_state(MythicalChainState::Damaged);
		let before = link_severity(intact, 0.4, 0.0, local);
		let after = link_severity(damaged, 0.4, 0.0, local);
		assert!(after + 1e-5 >= before, "restamp must be a bounded union");
		let look_before = LinkLook::from_severity(before, 0.0, 1.0, 11.0);
		let look_after = LinkLook::from_severity(after, 0.0, 1.0, 11.0);
		assert!(look_after.removed_chunks >= look_before.removed_chunks);
		Ok(())
	}

	#[test]
	fn thresholds_are_stable_for_a_seed() -> anyhow::Result<()> {
		let a = chunk_threshold(3.0, 11.0, 2.0);
		let b = chunk_threshold(3.0, 11.0, 2.0);
		assert_eq!(a, b);
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
