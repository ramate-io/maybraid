//! Noise-driven rectangular packing for a bedroom cell.
//!
//! Layouts stay inside the room AABB (wall bounds) and avoid circulation
//! exclusion volumes projected inward from outstanding door/opening regions.

use bevy_math::bounding::Aabb3d;
use bevy_math::Vec3;
use procedural_common::{NoiseConfig, NoiseParams};

use crate::constraints::circulation::aabb3d_intersects;
use crate::CellConstraints;

/// Child AABBs inside a room footprint.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BedroomLayout {
	pub closet: Aabb3d,
	pub ensuite: Aabb3d,
	pub bed: Aabb3d,
	pub nightstand: Aabb3d,
}

#[derive(Debug, Clone, Copy)]
struct Topology {
	/// Closet along −Z (`true`) or +Z (`false`).
	closet_on_front: bool,
	/// Ensuite along +X (`true`) or −X (`false`).
	ensuite_on_right: bool,
	closet_depth_frac: f32,
	ensuite_depth_frac: f32,
	bed_u: f32,
	bed_v: f32,
}

impl BedroomLayout {
	/// Fit closet / ensuite / bed / nightstand into `constraints` using `noise`.
	///
	/// Tries several noise-derived topologies and picks the first whose child
	/// AABBs miss circulation exclusion zones; falls back to the least-bad pack.
	pub fn fit(constraints: &CellConstraints, noise: NoiseParams) -> Self {
		let room = &constraints.aabb;
		let exclusions = constraints.circulation_exclusion_zones();
		let cfg = NoiseConfig::new(noise);

		let mut best: Option<(Self, u32)> = None;
		for attempt in 0..8u32 {
			let topo = Topology::sample(&cfg, attempt);
			let layout = Self::pack(room, topo);
			let hits = layout.exclusion_hits(&exclusions);
			if hits == 0 {
				return layout;
			}
			match &best {
				Some((_, best_hits)) if hits >= *best_hits => {}
				_ => best = Some((layout, hits)),
			}
		}
		best.map(|(layout, _)| layout)
			.unwrap_or_else(|| Self::pack(room, Topology::default_pack()))
	}

	/// Deterministic pack with no noise (closet −Z, ensuite +X). Prefer [`Self::fit`].
	pub fn from_room_aabb(room: &Aabb3d) -> Self {
		Self::pack(room, Topology::default_pack())
	}

	fn exclusion_hits(&self, exclusions: &[Aabb3d]) -> u32 {
		let children = [self.closet, self.ensuite, self.bed, self.nightstand];
		let mut hits = 0u32;
		for child in children {
			for zone in exclusions {
				if aabb3d_intersects(&child, zone) {
					hits += 1;
				}
			}
		}
		hits
	}

	fn pack(room: &Aabb3d, topo: Topology) -> Self {
		let min = room.min;
		let max = room.max;
		let size = max - min;

		let closet_depth = (0.75_f32)
			.min(size.z * topo.closet_depth_frac)
			.max(0.45)
			.min(size.z * 0.45);
		let ensuite_depth = (1.1_f32)
			.min(size.x * topo.ensuite_depth_frac)
			.max(0.7)
			.min(size.x * 0.45);

		let (living_x0, living_x1, ensuite) = if topo.ensuite_on_right {
			(
				min.x,
				max.x - ensuite_depth,
				Aabb3d::from_min_max(
					Vec3::new(max.x - ensuite_depth, min.y, min.z),
					Vec3::new(max.x, max.y, max.z),
				),
			)
		} else {
			(
				min.x + ensuite_depth,
				max.x,
				Aabb3d::from_min_max(
					Vec3::new(min.x, min.y, min.z),
					Vec3::new(min.x + ensuite_depth, max.y, max.z),
				),
			)
		};

		let (living_z0, living_z1, closet) = if topo.closet_on_front {
			(
				min.z + closet_depth,
				max.z,
				Aabb3d::from_min_max(
					Vec3::new(living_x0, min.y, min.z),
					Vec3::new(living_x1, max.y, min.z + closet_depth),
				),
			)
		} else {
			(
				min.z,
				max.z - closet_depth,
				Aabb3d::from_min_max(
					Vec3::new(living_x0, min.y, max.z - closet_depth),
					Vec3::new(living_x1, max.y, max.z),
				),
			)
		};

		let living_min = Vec3::new(living_x0, min.y, living_z0);
		let living_max = Vec3::new(living_x1, max.y, living_z1);
		let living = (living_max - living_min).max(Vec3::splat(1e-4));

		let bed_w = (2.0_f32).min(living.x * 0.55).max(1.2).min(living.x * 0.9);
		let bed_d = (1.6_f32).min(living.z * 0.55).max(1.0).min(living.z * 0.9);
		let bed_h = (0.55_f32).min(living.y * 0.35).max(0.35);
		let max_u = (living.x - bed_w).max(0.0);
		let max_v = (living.z - bed_d).max(0.0);
		let bed_min = Vec3::new(
			living_min.x + topo.bed_u.clamp(0.0, 1.0) * max_u,
			living_min.y,
			living_min.z + topo.bed_v.clamp(0.0, 1.0) * max_v,
		);
		let bed = Aabb3d::from_min_max(bed_min, bed_min + Vec3::new(bed_w, bed_h, bed_d));

		let ns = 0.45_f32.min(living.x * 0.25).max(0.3);
		let ns_gap = 0.08_f32;
		let ns_x = if bed.max.x + ns_gap + ns <= living_max.x - 0.05 {
			bed.max.x + ns_gap
		} else {
			(bed.min.x - ns_gap - ns).max(living_min.x + 0.05)
		};
		let ns_min = Vec3::new(
			ns_x,
			living_min.y,
			(bed.min.z + bed_d * 0.5 - ns * 0.5).clamp(living_min.z, living_max.z - ns),
		);
		let nightstand = Aabb3d::from_min_max(
			ns_min,
			Vec3::new(
				(ns_min.x + ns).min(living_max.x),
				living_min.y + 0.5_f32.min(living.y * 0.4),
				(ns_min.z + ns).min(living_max.z),
			),
		);

		Self {
			closet,
			ensuite,
			bed,
			nightstand,
		}
	}
}

impl Topology {
	fn default_pack() -> Self {
		Self {
			closet_on_front: true,
			ensuite_on_right: true,
			closet_depth_frac: 0.35,
			ensuite_depth_frac: 0.35,
			bed_u: 0.15,
			bed_v: 0.5,
		}
	}

	fn sample(noise: &NoiseConfig, attempt: u32) -> Self {
		let a = attempt as f32;
		let closet_on_front = noise.sample_unit_4d(a, 0.0, 0.0, 1.0) >= 0.5;
		let ensuite_on_right = noise.sample_unit_4d(a, 0.0, 0.0, 2.0) >= 0.5;
		// Flip axes across attempts so we explore mirrored packs even when noise is flat.
		let closet_on_front = closet_on_front ^ (attempt % 2 == 1);
		let ensuite_on_right = ensuite_on_right ^ ((attempt / 2) % 2 == 1);
		Self {
			closet_on_front,
			ensuite_on_right,
			closet_depth_frac: noise.sample_range_f32_4d(0.28, 0.4, a, 0.0, 0.0, 3.0),
			ensuite_depth_frac: noise.sample_range_f32_4d(0.28, 0.4, a, 0.0, 0.0, 4.0),
			bed_u: noise.sample_unit_4d(a, 0.0, 0.0, 5.0),
			bed_v: noise.sample_unit_4d(a, 0.0, 0.0, 6.0),
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use bevy_math::{bounding::Aabb2d, Vec2};
	use crate::constraints::{CirculationEntry, CirculationRequestStatus};

	#[test]
	fn fit_avoids_front_door_exclusion() -> anyhow::Result<()> {
		let mut cell = CellConstraints::cell_owned(Aabb3d::from_min_max(
			Vec3::ZERO,
			Vec3::new(5.0, 3.0, 5.0),
		));
		// Narrow centered door on −Z so side ensuite + back closet can clear.
		cell.circulation.front = Some(CirculationEntry(vec![(
			Aabb2d {
				min: Vec2::new(0.4, 0.0),
				max: Vec2::new(0.55, 0.9),
			},
			vec![CirculationRequestStatus::Required],
		)]));
		let layout = BedroomLayout::fit(
			&cell,
			NoiseParams {
				seed: 42,
				..NoiseParams::default()
			},
		);
		let zones = cell.circulation_exclusion_zones();
		assert!(!zones.is_empty());
		assert_eq!(layout.exclusion_hits(&zones), 0);
		assert!(layout.closet.min.z > zones[0].max.z - 1e-3);
		Ok(())
	}
}
