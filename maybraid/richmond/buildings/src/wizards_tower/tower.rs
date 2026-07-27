//! The stacked circular column of the Wizard's Tower.

use bevy::scene::prelude::Scene;
use bevy_math::bounding::Aabb3d;
use bevy_math::Vec3;
use lod::gen::LodScene;
use lod::lod_ref::LodRef;
use procedural_common::NoiseParams;
use richmond_building_components::scene_children;

use crate::wizards_tower::floor_fill::WALL_HEIGHT_METERS;
use crate::wizards_tower::{WizardsTowerFloor, WizardsTowerPerch};
use crate::CellConstraints;

/// Vertical stack of tower floors capped by a perch.
#[derive(Debug, Clone, PartialEq)]
pub struct WizardsTowerColumn {
	pub constraints: CellConstraints,
	/// Storey height in meters (outer ring wall \(Y\) scale; floor spacing).
	pub storey_height: f32,
	pub floors: Vec<WizardsTowerFloor>,
	pub perch: WizardsTowerPerch,
}

impl WizardsTowerColumn {
	/// Build from the tower footprint constraints, floor count, storey height, and
	/// a base portal [`NoiseParams`] (per-storey seeds are derived from this).
	///
	/// Each regular storey occupies \([y_i, y_i + h)\) with \(h =\) `storey_height`.
	/// The perch sits in the next slab of the same height. Footprint \(XZ\) and base
	/// \(Y\) come from `tower_constraints`; the parent AABB's max \(Y\) is ignored.
	pub fn new(
		tower_constraints: &CellConstraints,
		floor_count: u32,
		storey_height: f32,
		portal_noise: NoiseParams,
	) -> Self {
		let storey_height = storey_height.max(1e-4);
		let floor_count = floor_count.max(1);

		let min_y = tower_constraints.aabb.min.y;
		let floors_top = min_y + floor_count as f32 * storey_height;
		let perch_top = floors_top + storey_height;

		// Rebuild write AABB to match the stacked height (XZ from the footprint).
		let footprint = Aabb3d::from_min_max(
			Vec3::new(
				tower_constraints.aabb.min.x,
				min_y,
				tower_constraints.aabb.min.z,
			),
			Vec3::new(
				tower_constraints.aabb.max.x,
				perch_top,
				tower_constraints.aabb.max.z,
			),
		);
		let constraints = tower_constraints
			.subset(footprint)
			.unwrap_or_else(|_| CellConstraints::cell_owned(footprint));

		let floors = (0..floor_count)
			.map(|i| {
				let y0 = min_y + i as f32 * storey_height;
				let y1 = y0 + storey_height;
				let floor_aabb = Self::vertical_slab(&constraints.aabb, y0, y1);
				let floor_constraints = constraints
					.subset(floor_aabb)
					.unwrap_or_else(|_| CellConstraints::cell_owned(floor_aabb));
				let mut floor_noise = portal_noise;
				floor_noise.seed = portal_noise.seed.wrapping_add(i as i32 * 97);
				WizardsTowerFloor::new(&constraints, floor_constraints, storey_height, floor_noise)
			})
			.collect();

		let perch_aabb = Self::vertical_slab(&constraints.aabb, floors_top, perch_top);
		let perch_constraints = constraints
			.subset(perch_aabb)
			.unwrap_or_else(|_| CellConstraints::cell_owned(perch_aabb));
		let mut perch_noise = portal_noise;
		perch_noise.seed = portal_noise.seed.wrapping_add(floor_count as i32 * 97 + 13);
		let perch =
			WizardsTowerPerch::new(&constraints, perch_constraints, storey_height, perch_noise);

		Self {
			constraints,
			storey_height,
			floors,
			perch,
		}
	}

	/// Same as [`Self::new`] with [`WALL_HEIGHT_METERS`].
	pub fn with_default_storey_height(
		tower_constraints: &CellConstraints,
		floor_count: u32,
		portal_noise: NoiseParams,
	) -> Self {
		Self::new(
			tower_constraints,
			floor_count,
			WALL_HEIGHT_METERS,
			portal_noise,
		)
	}

	fn vertical_slab(parent: &Aabb3d, y_min: f32, y_max: f32) -> Aabb3d {
		Aabb3d::from_min_max(
			Vec3::new(parent.min.x, y_min, parent.min.z),
			Vec3::new(parent.max.x, y_max, parent.max.z),
		)
	}
}

impl LodScene for WizardsTowerColumn {
	fn scene_with_lod(&self, lod_ref: &LodRef) -> impl Scene + 'static {
		let mut children: Vec<Box<dyn Scene>> = self
			.floors
			.iter()
			.map(|floor| Box::new(floor.scene_with_lod(lod_ref)) as Box<dyn Scene>)
			.collect();
		children.push(Box::new(self.perch.scene_with_lod(lod_ref)));
		scene_children(children)
	}
}
