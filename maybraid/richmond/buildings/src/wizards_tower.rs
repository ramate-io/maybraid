//! The Wizard's Tower has between 10 and 30 floors of circular tower columns,
//! with a larger perch on the top floor.
//!
//! Floor count is derived from noise at construction time.
//!
//! # Layering (current sketch)
//!
//! Each storey draws circular **outer rings** and a squared-off **floor** (circle
//!−inscribed-square caps + rectangular slabs around a spire hole). Internal
//! partitions, rooms, and spire geometry are deferred.

pub mod floor;
pub mod floor_fill;
pub mod perch;
pub mod room;
pub mod spire;
pub mod tower;

pub use floor::WizardsTowerFloor;
pub use perch::WizardsTowerPerch;
pub use room::WizardsTowerRoom;
pub use spire::WizardsTowerSpire;
pub use tower::WizardsTowerColumn;

use bevy::scene::prelude::Scene;
use lod::gen::LodScene;
use lod::lod_ref::LodRef;

use richmond_building_components::scene_children;

use crate::CellConstraints;

/// Root authored building: a circular tower column stack with a central spire
/// and a larger perch on the top floor.
#[derive(Debug, Clone, PartialEq)]
pub struct WizardsTower {
	/// Generation / write constraints for the whole footprint.
	pub constraints: CellConstraints,
	/// Number of regular floors derived from noise (`10..=30`).
	pub floor_count: u32,
	/// The stacked circular column (floors + top perch).
	pub column: WizardsTowerColumn,
}

impl WizardsTower {
	/// Build from footprint constraints and a unit noise sample in \([0, 1]\).
	pub fn new(constraints: &CellConstraints, noise: f32) -> Self {
		let floor_count = Self::floor_count_from_noise(noise);
		let column = WizardsTowerColumn::new(constraints, floor_count);
		Self {
			constraints: constraints.clone(),
			floor_count,
			column,
		}
	}

	/// Map unit noise to floor count in `10..=30`.
	pub fn floor_count_from_noise(noise: f32) -> u32 {
		let t = noise.clamp(0.0, 1.0);
		10 + (t * 20.0).round() as u32
	}
}

impl LodScene for WizardsTower {
	fn scene_with_lod(&self, lod_ref: &LodRef) -> impl Scene + 'static {
		let column = self.column.scene_with_lod(lod_ref);
		scene_children(vec![Box::new(column) as Box<dyn Scene>])
	}
}
