//! Staged structural deterioration. CPU port of the WGSL bite model.
//!
//! Decorative energy conduits are not fractures. Each link has a handful of
//! irregular rest-space **bites** with stable centers and damage thresholds.
//! A bite grows as severity rises and never shrinks. Threshold `0` is unused
//! here: reserved metal is the complement of the bites, kept until release.
//! Severity is `max(global, local)` — flash and flow never enter it.
//! Weakness may shift global onset, but a local `1.0` is terminal for every
//! seed.
//!
//! Representation: four **through-cut sectors** along the link (`along_link`,
//! V wraps). A sector ignores `around` so it opens the hoop silhouette instead
//! of nicking the back face. Bite 0 is a chip; bite 1 sits next to it so mid
//! damage reads as one larger wound; bites 2–3 open the opposite side and a
//! late gap. Cost is four hashes plus four wrap-aware 1-D tests per fragment.

use crate::chain::hash::{hash13, link_weakness as hash_weakness};
use crate::chain::params::MythicalChainScalars;

/// Bites carved from each link. One reserved complement stays until fade.
pub const BITES_PER_LINK: u32 = 4;
/// Older name kept so existing `use` sites compile.
pub const CHUNKS_PER_LINK: u32 = BITES_PER_LINK;

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

/// Onset for bite `i`. Rank is the bite index so early / mid / late stay
/// distinct regardless of seed.
pub fn bite_threshold(bite: u32) -> f32 {
	match bite {
		0 => 0.18,
		1 => 0.36,
		2 => 0.58,
		3 => 0.80,
		_ => 0.0,
	}
}

/// Older name: threshold for a bite id.
pub fn chunk_threshold(_link_id: f32, _seed: f32, chunk_id: f32) -> f32 {
	bite_threshold(chunk_id as u32)
}

fn bite_half_range(bite: u32) -> (f32, f32) {
	match bite {
		0 => (0.045, 0.070),
		1 => (0.075, 0.115),
		2 => (0.085, 0.135),
		3 => (0.110, 0.180),
		_ => (0.0, 0.0),
	}
}

/// Rest-UV center of bite `i`. Only V (along the hoop) matters; U is unused
/// because the cut goes through the tube. Bite 1 sits next to bite 0.
pub fn bite_center(link_id: f32, seed: f32, bite: u32) -> [f32; 2] {
	let origin = hash13([0.5, seed * 0.017, link_id * 0.31]);
	let v = match bite {
		0 => origin,
		1 => (origin + 0.11).rem_euclid(1.0),
		2 => (origin + 0.50).rem_euclid(1.0),
		3 => (origin + 0.68).rem_euclid(1.0),
		_ => 0.5,
	};
	[0.5, v]
}

/// Current sector half-width along V. Zero before onset; grows; never shrinks.
pub fn bite_radius(severity: f32, bite: u32) -> f32 {
	let threshold = bite_threshold(bite);
	if threshold <= 0.0 || severity + 1e-5 < threshold {
		return 0.0;
	}
	let (start, end) = bite_half_range(bite);
	let t = ((severity - threshold) / 0.22).clamp(0.0, 1.0);
	start + (end - start) * t
}

fn wrap_delta(a: f32, b: f32) -> f32 {
	let d = a - b;
	d - d.round()
}

/// Wrapped along-link distance to a bite center.
pub fn bite_distance(local_uv: [f32; 2], center: [f32; 2]) -> f32 {
	wrap_delta(local_uv[1], center[1]).abs()
}

/// True when this rest-UV sample is inside an active through-cut, or fade completed.
pub fn point_removed(
	local_uv: [f32; 2],
	link_id: f32,
	seed: f32,
	severity: f32,
	fade: f32,
) -> bool {
	if fade >= 0.995 {
		return true;
	}
	for bite in 0..BITES_PER_LINK {
		let half = bite_radius(severity, bite);
		if half <= 0.0 {
			continue;
		}
		if bite_distance(local_uv, bite_center(link_id, seed, bite)) < half {
			return true;
		}
	}
	false
}

/// A region is gone when fade completes or severity crosses its threshold.
/// For bites, “region” means the bite’s current disk.
pub fn region_removed(severity: f32, threshold: f32, fade: f32) -> bool {
	if fade >= 0.995 {
		return true;
	}
	if threshold <= 0.0 {
		return fade > 0.82;
	}
	severity + 1e-5 >= threshold
}

/// One link’s structural look for tests and docs. Not a shading formula.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LinkLook {
	pub severity: f32,
	pub removed_chunks: u32,
	pub reserved_remaining: u32,
}

impl LinkLook {
	pub fn from_severity(severity: f32, fade: f32, _link_id: f32, _seed: f32) -> Self {
		let mut removed = 0u32;
		for bite in 0..BITES_PER_LINK {
			if region_removed(severity, bite_threshold(bite), fade) {
				removed += 1;
			}
		}
		let reserved = if fade >= 0.995 { 0 } else { 1 };
		Self { severity, removed_chunks: removed, reserved_remaining: reserved }
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::chain::params::{MythicalChainScalars, MythicalChainState};

	#[test]
	fn weakness_is_stable_and_in_unit_range() -> anyhow::Result<()> {
		let a = link_weakness(3, 11);
		let b = link_weakness(3, 11);
		assert!((a - b).abs() < 1e-6);
		assert!((0.0..=1.0).contains(&a));
		assert_ne!(link_weakness(3, 11), link_weakness(4, 11));
		assert_ne!(link_weakness(3, 11), link_weakness(3, 29));
		assert_eq!(a, crate::chain::hash::link_weakness(3.0, 11.0));
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
				look.removed_chunks, BITES_PER_LINK,
				"terminal local damage must open every bite"
			);
			assert!(look.reserved_remaining > 0, "complement metal survives until fade");
			let mut survived = 0u32;
			for i in 0..32 {
				let v = i as f32 / 32.0;
				if !point_removed([0.5, v], 3.0, 11.0, value, 0.0) {
					survived += 1;
				}
			}
			assert!(survived >= 6, "reserved complement should remain, survived={survived}");
		}
		Ok(())
	}

	#[test]
	fn increasing_damage_never_restores_chunks() -> anyhow::Result<()> {
		let seed = 11.0;
		let link = 4.0;
		let mut previous = 0u32;
		let mut previous_area = 0.0_f32;
		for step in 0..=20 {
			let damage = step as f32 / 20.0;
			let sev = severity(damage, 0.0, 0.4);
			let look = LinkLook::from_severity(sev, 0.0, link, seed);
			assert!(
				look.removed_chunks >= previous,
				"damage {damage} restored bites {previous} → {}",
				look.removed_chunks
			);
			previous = look.removed_chunks;
			let mut area = 0.0;
			for bite in 0..BITES_PER_LINK {
				area += bite_radius(sev, bite);
			}
			assert!(
				area + 1e-5 >= previous_area,
				"damage {damage} shrank bite radii {previous_area} → {area}"
			);
			previous_area = area;
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
		assert_eq!(full.removed_chunks, BITES_PER_LINK);
		assert!(full.reserved_remaining > 0);
		assert_eq!(LinkLook::from_severity(1.0, 1.0, link, seed).removed_chunks, BITES_PER_LINK);
		assert_eq!(LinkLook::from_severity(1.0, 1.0, link, seed).reserved_remaining, 0);
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
		let a = bite_center(3.0, 11.0, 2);
		let b = bite_center(3.0, 11.0, 2);
		assert_eq!(a, b);
		assert_eq!(bite_threshold(2), chunk_threshold(3.0, 11.0, 2.0));
		Ok(())
	}

	#[test]
	fn mid_damage_covers_a_substantial_wound() -> anyhow::Result<()> {
		let sev = severity(0.5, 0.0, 0.5);
		let center = bite_center(2.0, 11.0, 0);
		assert!(
			point_removed(center, 2.0, 11.0, sev, 0.0),
			"the first bite center must be gone by mid damage"
		);
		assert!(
			!point_removed([0.0, 0.92], 2.0, 11.0, sev, 0.0)
				|| !point_removed([0.5, 0.05], 2.0, 11.0, sev, 0.0),
			"mid damage must not erase the whole link"
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
