//! Central circular spire region of a Wizard's Tower floor.
//!
//! The parent floor passes a rectangular write region that circumscribes the
//! spire radius via [`CellConstraints::subset`](crate::CellConstraints::subset).
//! The spire scheme has exclusive rights to draw boundaries inside that region
//! (column core, spiral circulation, etc.).

use bevy::scene::prelude::Scene;
use lod::gen::LodScene;
use lod::lod_ref::LodRef;
use richmond_building_components::floors::RoughStoneFloorStructFill;
use richmond_building_components::partitions::rough_stonework::RoughStonework90;
use richmond_building_components::roofs::RoughStoneSpireRoof;
use richmond_building_components::stairs::RoughStoneSpiralStair;

use crate::wizards_tower::compose_scene;
use crate::CellConstraints;

/// Spire cell with exclusive boundary rights in its write bounds.
#[derive(Debug, Clone, PartialEq)]
pub struct WizardsTowerSpire {
	pub constraints: CellConstraints,
	pub core_quarters: [RoughStonework90; 4],
	pub struct_fill: RoughStoneFloorStructFill,
	pub spiral: RoughStoneSpiralStair,
	pub roof: RoughStoneSpireRoof,
}

impl WizardsTowerSpire {
	/// Build from floor/perch parent constraints and this spire's subsetted constraints.
	pub fn new(_parent_constraints: &CellConstraints, constraints: CellConstraints) -> Self {
		Self {
			constraints,
			core_quarters: [
				RoughStonework90,
				RoughStonework90,
				RoughStonework90,
				RoughStonework90,
			],
			struct_fill: RoughStoneFloorStructFill,
			spiral: RoughStoneSpiralStair,
			roof: RoughStoneSpireRoof,
		}
	}
}

impl LodScene for WizardsTowerSpire {
	fn scene_with_lod(&self, lod_ref: &LodRef) -> impl Scene + 'static {
		let mut children: Vec<Box<dyn Scene>> = self
			.core_quarters
			.iter()
			.map(|q| Box::new(q.scene_with_lod(lod_ref)) as Box<dyn Scene>)
			.collect();
		children.push(Box::new(self.struct_fill.scene_with_lod(lod_ref)));
		children.push(Box::new(self.spiral.scene_with_lod(lod_ref)));
		children.push(Box::new(self.roof.scene_with_lod(lod_ref)));
		compose_scene(children)
	}
}
