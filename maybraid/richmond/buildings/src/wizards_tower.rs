//! The Wizard's Tower has between 10 and 30 floors of circular tower columns,
//! with a larger perch on the top floor.
//!
//! Floor count is derived from noise at construction time.
//!
//! # Layering
//!
//! Each floor is an external rectangular cell that:
//! 1. Draws the circular outer walls.
//! 2. Draws up to four internally subdividing walls ending around a center
//!    radius reserved for the spire.
//! 3. Passes [`CellConstraints`](crate::CellConstraints) subsets to children:
//!    - **Spire rectangle** — circumscribes the spire radius; exclusive draw rights.
//!    - **Voxel halfspaces** — room-like regions around the spire for lower layers.

pub mod floor;
pub mod perch;
pub mod room;
pub mod spire;
pub mod tower;

pub use floor::WizardsTowerFloor;
pub use perch::WizardsTowerPerch;
pub use room::WizardsTowerRoom;
pub use spire::WizardsTowerSpire;
pub use tower::WizardsTowerColumn;

use bevy::scene::prelude::{bsn, Scene};
use bevy_math::bounding::Aabb3d;
use bevy_math::Vec3;
use lod::gen::LodScene;
use lod::lod_ref::LodRef;

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
		let floor_count = floor_count_from_noise(noise);
		let column = WizardsTowerColumn::new(constraints, floor_count);
		Self {
			constraints: constraints.clone(),
			floor_count,
			column,
		}
	}
}

impl LodScene for WizardsTower {
	fn scene_with_lod(&self, lod_ref: &LodRef) -> impl Scene + 'static {
		let column = self.column.scene_with_lod(lod_ref);
		bsn! {
			Children [ ({column}) ]
		}
	}
}

/// Map unit noise to floor count in `10..=30`.
pub fn floor_count_from_noise(noise: f32) -> u32 {
	let t = noise.clamp(0.0, 1.0);
	10 + (t * 20.0).round() as u32
}

/// Compose owned component / child scenes under one root.
pub(crate) fn compose_scene(children: Vec<Box<dyn Scene>>) -> impl Scene + 'static {
	bsn! {
		Children [ {children} ]
	}
}

/// Axis-aligned slab helpers used when subsetting the column into floors.
pub(crate) fn vertical_slab(parent: &Aabb3d, y_min: f32, y_max: f32) -> Aabb3d {
	Aabb3d::from_min_max(
		Vec3::new(parent.min.x, y_min, parent.min.z),
		Vec3::new(parent.max.x, y_max, parent.max.z),
	)
}

/// Center square circumscribing the spire radius (fraction of the floor footprint).
pub(crate) fn spire_rect(floor: &Aabb3d, half_extent_frac: f32) -> Aabb3d {
	let center = (floor.min + floor.max) * 0.5;
	let half = (floor.max - floor.min) * 0.5 * half_extent_frac;
	Aabb3d::from_min_max(
		Vec3::new(center.x - half.x, floor.min.y, center.z - half.z),
		Vec3::new(center.x + half.x, floor.max.y, center.z + half.z),
	)
}

/// Four voxel halfspaces around the spire (N/E/S/W), clipped to the floor.
pub(crate) fn voxel_halfspaces(floor: &Aabb3d, spire: &Aabb3d) -> [Aabb3d; 4] {
	[
		// -Z (front)
		Aabb3d::from_min_max(
			Vec3::new(spire.min.x, floor.min.y, floor.min.z),
			Vec3::new(spire.max.x, floor.max.y, spire.min.z),
		),
		// +X (right)
		Aabb3d::from_min_max(
			Vec3::new(spire.max.x, floor.min.y, spire.min.z),
			Vec3::new(floor.max.x, floor.max.y, spire.max.z),
		),
		// +Z (back)
		Aabb3d::from_min_max(
			Vec3::new(spire.min.x, floor.min.y, spire.max.z),
			Vec3::new(spire.max.x, floor.max.y, floor.max.z),
		),
		// -X (left)
		Aabb3d::from_min_max(
			Vec3::new(floor.min.x, floor.min.y, spire.min.z),
			Vec3::new(spire.min.x, floor.max.y, spire.max.z),
		),
	]
}
