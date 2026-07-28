//! Deterministic rectangular packing for a bedroom cell.

use bevy_math::bounding::Aabb3d;
use bevy_math::Vec3;

/// Child AABBs inside a room footprint.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BedroomLayout {
	pub closet: Aabb3d,
	pub ensuite: Aabb3d,
	pub bed: Aabb3d,
	pub nightstand: Aabb3d,
}

impl BedroomLayout {
	/// Pack closet (−Z), ensuite (+X), bed / nightstand in the remaining floor.
	pub fn from_room_aabb(room: &Aabb3d) -> Self {
		let min = room.min;
		let max = room.max;
		let size = max - min;
		let closet_depth = (0.75_f32).min(size.z * 0.35).max(0.45);
		let ensuite_depth = (1.1_f32).min(size.x * 0.35).max(0.7);

		let closet = Aabb3d::from_min_max(
			Vec3::new(min.x, min.y, min.z),
			Vec3::new(max.x - ensuite_depth, max.y, min.z + closet_depth),
		);

		let ensuite = Aabb3d::from_min_max(
			Vec3::new(max.x - ensuite_depth, min.y, min.z),
			Vec3::new(max.x, max.y, max.z),
		);

		let living_min = Vec3::new(min.x, min.y, min.z + closet_depth);
		let living_max = Vec3::new(max.x - ensuite_depth, max.y, max.z);
		let living = living_max - living_min;

		let bed_w = (2.0_f32).min(living.x * 0.55).max(1.2);
		let bed_d = (1.6_f32).min(living.z * 0.55).max(1.0);
		let bed_h = (0.55_f32).min(living.y * 0.35).max(0.35);
		let bed_min = Vec3::new(
			living_min.x + 0.25,
			living_min.y,
			living_min.z + (living.z - bed_d) * 0.5,
		);
		let bed = Aabb3d::from_min_max(bed_min, bed_min + Vec3::new(bed_w, bed_h, bed_d));

		let ns = 0.45_f32;
		let ns_min = Vec3::new(
			bed.max.x + 0.08,
			living_min.y,
			bed.min.z + bed_d * 0.5 - ns * 0.5,
		);
		let nightstand = Aabb3d::from_min_max(
			ns_min,
			Vec3::new(
				(ns_min.x + ns).min(living_max.x - 0.05),
				living_min.y + 0.5,
				ns_min.z + ns,
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
