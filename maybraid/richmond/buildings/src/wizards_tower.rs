//! The Wizard's Tower has between 10 and 30 floors of circular tower columns,
//! with a larger perch on the top floor.
//!
//! Floor count is derived from noise at construction time.
//!
//! # Layering (current sketch)
//!
//! Each storey draws a parameterized crate-level [`crate::ArcWall`] (must / must-not /
//! noise portals, configurable arc degrees), a crate-level [`crate::ArcSpire`] tread
//! run fitted to \(Y\) bindings, and a squared-off **floor** (circle−inscribed-square
//! caps + rectangular slabs around a spire hole). Internal partitions, rooms, and
//! spire geometry are deferred.

pub mod floor;
pub mod floor_fill;
pub mod perch;
pub mod room;
pub mod spire;
pub mod tower;
pub mod tower_lod;

pub use floor::WizardsTowerFloor;
pub use perch::WizardsTowerPerch;
pub use room::WizardsTowerRoom;
pub use spire::WizardsTowerSpire;
pub use tower::WizardsTowerColumn;
pub use tower_lod::{TowerLodBand, NEAR_RADIUS_MULTIPLIER};

use bevy::scene::prelude::Scene;
use lod::gen::{LodScene, LodSceneStatus};
use lod::lod_ref::LodRef;
use procedural_common::NoiseParams;

use richmond_building_components::scene_children;

use crate::arc_wall::{MustAssignPortal, Portal};
use crate::wizards_tower::floor_fill::WALL_HEIGHT_METERS;
use crate::wizards_tower::tower_lod::TowerLodFootprint;
use crate::CellConstraints;

/// Cardinal door + windows on a full 360° storey arc.
pub fn must_assign_cardinal_portals() -> Vec<MustAssignPortal> {
	vec![
		MustAssignPortal::at(0.0, Portal::Door),
		MustAssignPortal::at(0.25, Portal::Window),
		MustAssignPortal::at(0.5, Portal::Window),
		MustAssignPortal::at(0.75, Portal::Window),
	]
}

/// Root authored building: a circular tower column stack with a central spire
/// and a larger perch on the top floor.
#[derive(Debug, Clone, PartialEq)]
pub struct WizardsTower {
	/// Generation / write constraints for the whole footprint.
	pub constraints: CellConstraints,
	/// Number of regular floors derived from noise (`10..=30`).
	pub floor_count: u32,
	/// Storey height in meters (wall scale and vertical spacing).
	pub storey_height: f32,
	/// The stacked circular column (floors + top perch).
	pub column: WizardsTowerColumn,
}

impl WizardsTower {
	/// Build from footprint constraints and a unit noise sample in \([0, 1]\),
	/// using [`WALL_HEIGHT_METERS`] as the storey height.
	pub fn new(constraints: &CellConstraints, noise: f32) -> Self {
		Self::with_storey_height(constraints, noise, WALL_HEIGHT_METERS)
	}

	/// Build with an explicit storey / room height in meters.
	pub fn with_storey_height(
		constraints: &CellConstraints,
		noise: f32,
		storey_height: f32,
	) -> Self {
		let floor_count = Self::floor_count_from_noise(noise);
		let portal_noise = NoiseParams {
			seed: (noise.clamp(0.0, 1.0) * 1_000_000.0) as i32,
			..NoiseParams::default()
		};
		let column =
			WizardsTowerColumn::new(constraints, floor_count, storey_height, portal_noise);
		Self {
			constraints: column.constraints.clone(),
			floor_count,
			storey_height: column.storey_height,
			column,
		}
	}

	/// Map unit noise to floor count in `10..=30`.
	pub fn floor_count_from_noise(noise: f32) -> u32 {
		let t = noise.clamp(0.0, 1.0);
		10 + (t * 20.0).round() as u32
	}
}

impl TowerLodFootprint for WizardsTower {
	fn lod_aabb(&self) -> &bevy_math::bounding::Aabb3d {
		&self.constraints.aabb
	}
}

impl LodScene for WizardsTower {
	fn scene_lod_status(&self, lod_ref: &LodRef) -> LodSceneStatus {
		let prev = self.band_for(lod_ref.previous_transform);
		let curr = self.band_for(lod_ref.current_transform);
		if prev == curr {
			LodSceneStatus::Unchanged
		} else {
			LodSceneStatus::Changed
		}
	}

	fn scene_with_lod(&self, lod_ref: &LodRef) -> impl Scene + 'static {
		let column = self.column.scene_with_lod(lod_ref);
		scene_children(vec![Box::new(column) as Box<dyn Scene>])
	}
}
