//! The Wizard's Tower has between 10 and 30 floors of circular tower columns,
//! with a larger perch on the top floor.
//!
//! Floor count is derived from noise at construction time.
//!
//! LOD (root [`LodSceneHost`](lod::LodSceneHost)):
//! - **Low** — cylinder silhouette
//! - **Medium** — exterior walls
//! - **High** — exterior + internals (`ParentConfines::Internal` on nodes)

pub mod floor;
pub mod floor_fill;
pub mod perch;
pub mod room;
pub mod silhouette;
pub mod spire;
pub mod tower;
pub mod tower_lod;

pub use floor::WizardsTowerFloor;
pub use perch::WizardsTowerPerch;
pub use room::WizardsTowerRoom;
pub use silhouette::{TowerSilhouetteAssets, TowerSilhouettePlugin};
pub use spire::WizardsTowerSpire;
pub use tower::WizardsTowerColumn;
pub use tower_lod::{HIGH_FOOTPRINT_MULTIPLIER, LOW_RES_CUTOFF_METERS};

use bevy::prelude::{Component, Transform};
use bevy::scene::prelude::Scene;
use bevy_math::Vec3;
use lod::gen::{
	cull_offset_bands, LodScene, LodSceneCulls, LodSceneLevel, LodSceneStatus, SceneChunk,
};
use lod::lod_host_scene_pending;
use lod::lod_ref::LodRef;
use procedural_common::NoiseParams;

use richmond_building_components::floors::FloorNode;
use richmond_building_components::scene_children;
use richmond_building_components::stairs::StairNode;
use richmond_building_components::{
	append_component_scenes, BuildingComponents, Layers, PartitionNode,
};

use crate::portals::{MustAssignPortal, Portal};
use crate::wizards_tower::floor_fill::WALL_HEIGHT_METERS;
use crate::wizards_tower::silhouette::silhouette_scene;
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
#[derive(Debug, Clone, PartialEq, Component)]
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
		let column = WizardsTowerColumn::new(constraints, floor_count, storey_height, portal_noise);
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

	fn silhouette_transform(&self) -> Transform {
		let aabb = &self.constraints.aabb;
		let center = (aabb.min + aabb.max) * 0.5;
		let radius = self.footprint_radius().max(1e-4);
		let height = self.tower_height();
		Transform::from_translation(Vec3::new(center.x, aabb.min.y + height * 0.5, center.z))
			.with_scale(Vec3::new(radius * 2.0, height, radius * 2.0))
	}

	fn exterior_primitives(&self, lod_ref: &LodRef) -> impl Scene + 'static {
		let mut children: Vec<Box<dyn Scene>> = Vec::new();
		append_component_scenes(self, lod_ref, LodSceneLevel::Medium, &mut children);
		scene_children(children)
	}

	fn high_primitives(&self, lod_ref: &LodRef) -> impl Scene + 'static {
		let spire_confines = self.column.spire_confine_capsule();
		let mut children: Vec<Box<dyn Scene>> = Vec::new();
		for floor in &self.column.floors {
			floor.emit_external_features(&mut children, lod_ref);
			// Per-storey balls for slabs / lantern; one shaft capsule for all spire stairs.
			floor.emit_internal_features(&mut children, lod_ref);
			floor.emit_spire_features(&mut children, lod_ref, spire_confines);
		}
		self.column.perch.emit_external_features(&mut children, lod_ref);
		self.column.perch.emit_internal_features(&mut children, lod_ref);
		scene_children(children)
	}
}

impl BuildingComponents for WizardsTower {
	fn partition_nodes_for_level(&self, level: LodSceneLevel) -> Layers<PartitionNode> {
		self.column.partition_nodes_for_level(level)
	}

	fn floor_nodes_for_level(&self, level: LodSceneLevel) -> Layers<FloorNode> {
		self.column.floor_nodes_for_level(level)
	}

	fn stair_nodes_for_level(&self, level: LodSceneLevel) -> Layers<StairNode> {
		self.column.stair_nodes_for_level(level)
	}
}

impl TowerLodFootprint for WizardsTower {
	fn lod_aabb(&self) -> &bevy_math::bounding::Aabb3d {
		&self.constraints.aabb
	}
}

impl LodScene for WizardsTower {
	fn scene_lod_level(&self, lod_ref: &LodRef) -> LodSceneLevel {
		self.level_for_lod_ref(lod_ref)
	}

	fn scene_lod_status(&self, lod_ref: &LodRef) -> LodSceneStatus {
		let prev = self.level_for(lod_ref.previous_transform);
		let curr = self.level_for(lod_ref.current_transform);
		if prev == curr {
			LodSceneStatus::Unchanged
		} else {
			LodSceneStatus::Changed(curr)
		}
	}

	fn scene_lod_culls(&self, lod_ref: &LodRef, _current: LodSceneLevel) -> LodSceneCulls {
		// Offset bands: drop High once ~halfway into Medium; Low keeps Medium warm.
		let (level, progress) = self.band_progress_for_lod_ref(lod_ref);
		cull_offset_bands(level, progress).with_customs()
	}

	fn scene_with_level(&self, lod_ref: &LodRef, level: LodSceneLevel) -> impl Scene + 'static {
		match level {
			LodSceneLevel::High => Box::new(self.high_primitives(lod_ref)) as Box<dyn Scene>,
			LodSceneLevel::Medium => Box::new(self.exterior_primitives(lod_ref)) as Box<dyn Scene>,
			LodSceneLevel::Low
			| LodSceneLevel::UltraLow
			| LodSceneLevel::Distance(_)
			| LodSceneLevel::Resolution(_) => {
				let tf = self.silhouette_transform();
				Box::new(silhouette_scene(
					TowerSilhouetteAssets::cylinder(),
					TowerSilhouetteAssets::material(),
					tf,
				)) as Box<dyn Scene>
			}
		}
	}

	fn scene_chunks_with_level(&self, lod_ref: &LodRef, level: LodSceneLevel) -> SceneChunk {
		match level {
			LodSceneLevel::High => {
				let spire_confines = self.column.spire_confine_capsule();
				let mut chunks = Vec::new();
				for floor in &self.column.floors {
					let mut children: Vec<Box<dyn Scene>> = Vec::new();
					floor.emit_external_features(&mut children, lod_ref);
					floor.emit_internal_features(&mut children, lod_ref);
					floor.emit_spire_features(&mut children, lod_ref, spire_confines);
					chunks.push(SceneChunk::weighted(4, scene_children(children)));
				}
				let mut perch_children: Vec<Box<dyn Scene>> = Vec::new();
				self.column.perch.emit_external_features(&mut perch_children, lod_ref);
				self.column.perch.emit_internal_features(&mut perch_children, lod_ref);
				chunks.push(SceneChunk::weighted(3, scene_children(perch_children)));
				SceneChunk::chunks(chunks)
			}
			other => SceneChunk::primitive(self.scene_with_level(lod_ref, other)),
		}
	}

	fn scene_bounds(&self) -> bevy_math::bounding::Aabb3d {
		self.constraints.aabb
	}

	fn scene_with_lod(&self, lod_ref: &LodRef) -> impl Scene + 'static {
		let level = self.scene_lod_level(lod_ref);
		lod_host_scene_pending(level, self.constraints.aabb)
	}
}
