//! The stacked circular column of the Wizard's Tower.
//!
//! Owns the vertical sequence of floors and the top perch. The column does not
//! author wall geometry itself; each [`super::WizardsTowerFloor`] draws its ring
//! and subdividers, then hands subsetted [`CellConstraints`](crate::CellConstraints)
//! to spire / room children.

use bevy::scene::prelude::Scene;
use lod::gen::LodScene;
use lod::lod_ref::LodRef;

use crate::wizards_tower::{
	compose_scene, vertical_slab, WizardsTowerFloor, WizardsTowerPerch,
};
use crate::CellConstraints;

/// Vertical stack of tower floors capped by a perch.
#[derive(Debug, Clone, PartialEq)]
pub struct WizardsTowerColumn {
	pub constraints: CellConstraints,
	pub floors: Vec<WizardsTowerFloor>,
	pub perch: WizardsTowerPerch,
}

impl WizardsTowerColumn {
	/// Build from the tower footprint constraints and a derived floor count.
	pub fn new(tower_constraints: &CellConstraints, floor_count: u32) -> Self {
		let constraints = tower_constraints
			.subset(tower_constraints.aabb)
			.unwrap_or_else(|_| tower_constraints.clone());

		let min_y = constraints.aabb.min.y;
		let max_y = constraints.aabb.max.y;
		let height = (max_y - min_y).max(1e-4);
		// Reserve the top ~8% of the column for the perch.
		let perch_frac = 0.08;
		let floors_top = min_y + height * (1.0 - perch_frac);
		let floor_count = floor_count.max(1);
		let floor_h = (floors_top - min_y) / floor_count as f32;

		let floors = (0..floor_count)
			.map(|i| {
				let y0 = min_y + i as f32 * floor_h;
				let y1 = y0 + floor_h;
				let floor_aabb = vertical_slab(&constraints.aabb, y0, y1);
				let floor_constraints = constraints
					.subset(floor_aabb)
					.unwrap_or_else(|_| CellConstraints::cell_owned(floor_aabb));
				WizardsTowerFloor::new(&constraints, floor_constraints)
			})
			.collect();

		let perch_aabb = vertical_slab(&constraints.aabb, floors_top, max_y);
		let perch_constraints = constraints
			.subset(perch_aabb)
			.unwrap_or_else(|_| CellConstraints::cell_owned(perch_aabb));
		let perch = WizardsTowerPerch::new(&constraints, perch_constraints);

		Self {
			constraints,
			floors,
			perch,
		}
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
		compose_scene(children)
	}
}
