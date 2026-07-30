//! Stacked circular wall rings — a minimal building used to validate kit scaling.
//!
//! Each storey is two 180° rough-stone arcs placed at the ring base and scaled
//! from the normalized kit (radius \(1\), height \(1\)) to `(radius, floor_height)`.

pub mod ring;

pub use ring::StackedRing;

use bevy::scene::prelude::Scene;
use bevy_math::bounding::Aabb3d;
use bevy_math::Vec3;
use lod::gen::LodScene;
use lod::lod_ref::LodRef;
use richmond_building_components::scene_children;

use crate::CellConstraints;

/// Vertical stack of circular wall rings.
#[derive(Debug, Clone, PartialEq)]
pub struct StackedRings {
	pub constraints: CellConstraints,
	pub floor_count: u32,
	pub floor_height: f32,
	pub radius: f32,
	pub rings: Vec<StackedRing>,
}

impl StackedRings {
	/// Build `floor_count` rings of height `floor_height` and radius `radius`.
	///
	/// The footprint AABB is axis-aligned around the origin:
	/// \([-R, R] \times [0, N \cdot H] \times [-R, R]\).
	pub fn new(floor_count: u32, floor_height: f32, radius: f32) -> Self {
		let floor_count = floor_count.max(1);
		let floor_height = floor_height.max(1e-4);
		let radius = radius.max(1e-4);
		let total_height = floor_count as f32 * floor_height;
		let constraints = CellConstraints::cell_owned(Aabb3d::from_min_max(
			Vec3::new(-radius, 0.0, -radius),
			Vec3::new(radius, total_height, radius),
		));

		let rings = (0..floor_count)
			.map(|i| {
				let base_y = i as f32 * floor_height;
				StackedRing::new(base_y, floor_height, radius)
			})
			.collect();

		Self { constraints, floor_count, floor_height, radius, rings }
	}
}

impl LodScene for StackedRings {
	fn scene_lod_status(&self, _lod_ref: &LodRef) -> lod::gen::LodSceneStatus {
		lod::gen::LodSceneStatus::Unchanged
	}

	fn scene_with_level(
		&self,
		lod_ref: &LodRef,
		_level: lod::gen::LodSceneLevel,
	) -> impl Scene + 'static {
		// Flatten ring wrappers so every GLB sits under one Transform/Visibility root.
		let children: Vec<Box<dyn Scene>> = self
			.rings
			.iter()
			.flat_map(|ring| ring.outer_walls.iter())
			.map(|wall| Box::new(wall.scene_with_lod(lod_ref)) as Box<dyn Scene>)
			.collect();
		scene_children(children)
	}
}
