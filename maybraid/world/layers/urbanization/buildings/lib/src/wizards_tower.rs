//! The Wizard's Tower has between 10 and 30 floors of circular tower columns,
//! with a larger perch on the top floor.
//!
//! Floor count is derived from noise at construction time.
//!
//! LOD (root [`LodSceneHost`](lod::LodSceneHost)):
//! - **Low** — cylinder silhouette
//! - **Medium** — exterior walls plus circulation (stairs)
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
use lod::lod_host_scene_pending;
use lod::lod_ref::LodRef;
use lod::scene::{
	cull_offset_bands, LodScene, LodSceneCulls, LodSceneLevel, LodSceneStatus, SceneChunk,
};
use material_ref::MaterialRef;
use procedural_common::NoiseParams;

use building_components::floors::FloorNode;
use building_components::scene_children;
use building_components::stairs::StairNode;
use building_components::{
	append_flattened_component_scenes, BuildingComponents, Layers, PartitionNode,
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
	/// Finish stamped onto the exterior ring walls.
	pub wall_material: Option<MaterialRef>,
	/// Finish stamped onto high-detail room partitions.
	pub room_material: Option<MaterialRef>,
	/// World (or parent) pose used to map a world-space viewer into the local AABB.
	pub host_transform: Transform,
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
			wall_material: None,
			room_material: None,
			host_transform: Transform::IDENTITY,
		}
	}

	/// Stamp exterior shell walls and high-detail room partitions without changing LOD behavior.
	pub fn with_finish(mut self, wall: MaterialRef, room: MaterialRef) -> Self {
		self.column = self.column.with_materials(wall.clone(), room.clone());
		self.wall_material = Some(wall);
		self.room_material = Some(room);
		self
	}

	pub fn with_wall_material(self, wall: MaterialRef) -> Self {
		let room = self.room_material.clone().unwrap_or_else(|| wall.clone());
		self.with_finish(wall, room)
	}

	pub fn with_room_material(self, room: MaterialRef) -> Self {
		let wall = self.wall_material.clone().unwrap_or_else(|| room.clone());
		self.with_finish(wall, room)
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
		append_flattened_component_scenes(self, lod_ref, LodSceneLevel::Medium, &mut children);
		scene_children(children)
	}

	fn high_primitives(&self, lod_ref: &LodRef) -> impl Scene + 'static {
		let mut children: Vec<Box<dyn Scene>> = Vec::new();
		for floor in &self.column.floors {
			floor.emit_external_features_at_mesh_level(&mut children, lod_ref, LodSceneLevel::High);
			// Rooms and lantern. The deck comes from the Medium exterior flatten.
			// Spire stairs come from that flatten too, not a second High-only bake.
			floor.emit_internal_features(&mut children, lod_ref);
		}
		self.column.perch.emit_external_features_at_mesh_level(
			&mut children,
			lod_ref,
			LodSceneLevel::High,
		);
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
		self.level_for_world_lod_ref(lod_ref, &self.host_transform)
	}

	fn scene_lod_status(&self, lod_ref: &LodRef) -> LodSceneStatus {
		let prev =
			self.level_for_world(lod_ref.previous_transform.translation, &self.host_transform);
		let curr =
			self.level_for_world(lod_ref.current_transform.translation, &self.host_transform);
		if prev == curr {
			LodSceneStatus::Unchanged
		} else {
			LodSceneStatus::Changed(curr)
		}
	}

	fn scene_lod_culls(&self, lod_ref: &LodRef, _current: LodSceneLevel) -> LodSceneCulls {
		// Offset bands: drop High once ~halfway into Medium; Low keeps Medium warm.
		let (level, progress) = self.band_progress_for_world_lod_ref(lod_ref, &self.host_transform);
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
				let mut chunks = Vec::new();
				for floor in &self.column.floors {
					let mut children: Vec<Box<dyn Scene>> = Vec::new();
					floor.emit_external_features_at_mesh_level(
						&mut children,
						lod_ref,
						LodSceneLevel::High,
					);
					floor.emit_internal_features(&mut children, lod_ref);
					chunks.push(SceneChunk::weighted(4, scene_children(children)));
				}
				let mut perch_children: Vec<Box<dyn Scene>> = Vec::new();
				self.column.perch.emit_external_features_at_mesh_level(
					&mut perch_children,
					lod_ref,
					LodSceneLevel::High,
				);
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

#[cfg(test)]
mod tests {
	use bevy::prelude::{App, AssetPlugin, Children, Entity, TaskPoolPlugin, Transform};
	use bevy::scene::{ScenePlugin, WorldSceneExt};
	use bevy_math::bounding::Aabb3d;
	use bevy_math::Vec3;
	use material_ref::MaterialRefRoot;
	use scene_ref::SceneRefRoot;

	use super::*;

	fn tower() -> WizardsTower {
		let constraints = CellConstraints::cell_owned(Aabb3d::from_min_max(
			Vec3::new(-10.0, 0.0, -10.0),
			Vec3::new(10.0, 80.0, 10.0),
		));
		WizardsTower::new(&constraints, 0.0)
	}

	fn scene_refs_for_material(app: &App, material: &material_ref::MaterialRef) -> Vec<String> {
		let world = app.world();
		let roots: Vec<Entity> = world
			.iter_entities()
			.filter_map(|entity| {
				entity
					.get::<MaterialRefRoot>()
					.filter(|root| &root.0 == material)
					.map(|_| entity.id())
			})
			.collect();
		let mut paths = Vec::new();
		for root in roots {
			let mut pending = vec![root];
			while let Some(entity) = pending.pop() {
				if let Some(scene_ref) = world.get::<SceneRefRoot>(entity) {
					paths.push(scene_ref.0.path.clone());
				}
				if let Some(children) = world.get::<Children>(entity) {
					pending.extend(children.iter());
				}
			}
		}
		paths
	}

	fn scene_app() -> App {
		let mut app = App::new();
		app.add_plugins((
			TaskPoolPlugin::default(),
			AssetPlugin::default(),
			ScenePlugin::default(),
		));
		app
	}

	fn spawn_scene(app: &mut App, scene: impl bevy::scene::Scene + 'static) {
		assert!(app.world_mut().spawn_scene(scene).is_ok());
	}

	fn expected_room_scene_ref_count(tower: &WizardsTower, lod_ref: &LodRef) -> usize {
		let mut app = scene_app();
		for floor in &tower.column.floors {
			for room in &floor.rooms {
				for node in room.partition_nodes_for_level(LodSceneLevel::High).flatten() {
					spawn_scene(&mut app, node.scene_with_level(lod_ref, LodSceneLevel::High));
				}
			}
		}
		match &tower.room_material {
			Some(material) => scene_refs_for_material(&app, material).len(),
			None => 0,
		}
	}

	fn expected_shell_scene_ref_count(tower: &WizardsTower, lod_ref: &LodRef) -> usize {
		let mut app = scene_app();
		for floor in &tower.column.floors {
			for node in floor.partition_nodes_for_level(LodSceneLevel::Medium).flatten() {
				spawn_scene(&mut app, node.scene_with_level(lod_ref, LodSceneLevel::High));
			}
		}
		for node in tower.column.perch.partition_nodes_for_level(LodSceneLevel::Medium).flatten() {
			spawn_scene(&mut app, node.scene_with_level(lod_ref, LodSceneLevel::High));
		}
		match &tower.wall_material {
			Some(material) => scene_refs_for_material(&app, material).len(),
			None => 0,
		}
	}

	fn assert_tier(paths: &[String], suffix: &str) {
		assert!(!paths.is_empty());
		assert!(paths.iter().all(|path| path.contains(suffix)));
	}

	#[test]
	fn high_exterior_uses_high_meshes_without_duplicating_rooms() {
		let wall = MaterialRef::named("wizard-shell");
		let room = MaterialRef::named("wizard-room");
		let tower = tower().with_finish(wall.clone(), room.clone());
		let transform = Transform::IDENTITY;
		let bounds = Aabb3d::from_min_max(Vec3::ZERO, Vec3::ONE);
		let lod_ref = LodRef {
			entity: Entity::PLACEHOLDER,
			previous_transform: &transform,
			current_transform: &transform,
			bounds: &bounds,
		};
		let expected_rooms = expected_room_scene_ref_count(&tower, &lod_ref);
		let expected_shell = expected_shell_scene_ref_count(&tower, &lod_ref);
		assert!(expected_rooms > 0);
		assert!(expected_shell > 0);

		let mut high_app = scene_app();
		spawn_scene(&mut high_app, tower.scene_with_level(&lod_ref, LodSceneLevel::High));
		let high_shell = scene_refs_for_material(&high_app, &wall);
		assert_tier(&high_shell, "_high_res.");
		assert_eq!(high_shell.len(), expected_shell);
		assert_eq!(scene_refs_for_material(&high_app, &room).len(), expected_rooms);

		let mut medium_app = scene_app();
		spawn_scene(&mut medium_app, tower.scene_with_level(&lod_ref, LodSceneLevel::Medium));
		let medium_shell = scene_refs_for_material(&medium_app, &wall);
		assert_tier(&medium_shell, "_mid_res.");
		assert_eq!(medium_shell.len(), expected_shell);
		assert!(scene_refs_for_material(&medium_app, &room).is_empty());
	}

	#[test]
	fn high_chunks_use_high_exterior_meshes_without_duplicating_rooms() {
		let wall = MaterialRef::named("wizard-shell");
		let room = MaterialRef::named("wizard-room");
		let tower = tower().with_finish(wall.clone(), room.clone());
		let transform = Transform::IDENTITY;
		let bounds = Aabb3d::from_min_max(Vec3::ZERO, Vec3::ONE);
		let lod_ref = LodRef {
			entity: Entity::PLACEHOLDER,
			previous_transform: &transform,
			current_transform: &transform,
			bounds: &bounds,
		};
		let expected_rooms = expected_room_scene_ref_count(&tower, &lod_ref);
		let expected_shell = expected_shell_scene_ref_count(&tower, &lod_ref);
		assert!(expected_rooms > 0);
		assert!(expected_shell > 0);
		let mut app = scene_app();
		for (_, scene) in
			tower.scene_chunks_with_level(&lod_ref, LodSceneLevel::High).into_primitives()
		{
			spawn_scene(&mut app, scene);
		}
		let shell = scene_refs_for_material(&app, &wall);
		assert_tier(&shell, "_high_res.");
		assert_eq!(shell.len(), expected_shell);
		assert_eq!(scene_refs_for_material(&app, &room).len(), expected_rooms);
	}

	#[test]
	fn every_floor_has_non_overlapping_rooms_clear_of_shaft() {
		let tower = tower();
		for floor in &tower.column.floors {
			assert_eq!(floor.rooms.len(), 4);
			let center = (floor.constraints.aabb.min + floor.constraints.aabb.max) * 0.5;
			let radius = 0.5
				* (floor.constraints.aabb.max.x - floor.constraints.aabb.min.x)
					.min(floor.constraints.aabb.max.z - floor.constraints.aabb.min.z);
			let shaft_half = floor_fill::SPIRE_HALF_FRAC * radius;
			for (index, room) in floor.rooms.iter().enumerate() {
				let bounds = room.constraints.aabb;
				assert!(bounds.min.cmpge(floor.constraints.aabb.min).all());
				assert!(bounds.max.cmple(floor.constraints.aabb.max).all());
				let avoids_shaft = bounds.max.x <= center.x - shaft_half
					|| bounds.min.x >= center.x + shaft_half
					|| bounds.max.z <= center.z - shaft_half
					|| bounds.min.z >= center.z + shaft_half;
				assert!(avoids_shaft);
				for other in floor.rooms.iter().skip(index + 1) {
					let overlap_x = (bounds.max.x.min(other.constraints.aabb.max.x)
						- bounds.min.x.max(other.constraints.aabb.min.x))
					.max(0.0);
					let overlap_z = (bounds.max.z.min(other.constraints.aabb.max.z)
						- bounds.min.z.max(other.constraints.aabb.min.z))
					.max(0.0);
					assert!(overlap_x * overlap_z <= 1e-4);
				}
			}
		}
	}

	#[test]
	fn finishes_stamp_shell_and_high_only_rooms() {
		let wall = MaterialRef::named("wizard-shell");
		let room = MaterialRef::named("wizard-room");
		let tower = tower().with_finish(wall.clone(), room.clone());
		for floor in &tower.column.floors {
			let shell: Vec<_> = floor.partition_nodes_for_level(LodSceneLevel::Medium).flatten();
			assert!(!shell.is_empty());
			assert!(shell.iter().all(|node| node.material.as_ref() == Some(&wall)));
			assert!(floor
				.rooms
				.iter()
				.all(|authored_room| authored_room.wall_material.as_ref() == Some(&room)));
		}
		let high = tower.partition_nodes_for_level(LodSceneLevel::High).len();
		let medium = tower.partition_nodes_for_level(LodSceneLevel::Medium).len();
		let low = tower.partition_nodes_for_level(LodSceneLevel::Low).len();
		assert!(high > medium);
		assert_eq!(low, 0);
	}

	#[test]
	fn offset_tower_bands_from_the_world_viewer() {
		let mut tower = tower();
		tower.host_transform = Transform::from_xyz(2_000.0, 0.0, 0.0);
		let bounds = tower.scene_bounds();
		let beside = Transform::from_xyz(2_000.0, 1.5, 0.0);
		let far = Transform::from_xyz(1_400.0, 1.5, 0.0);
		let beside_ref = LodRef {
			entity: bevy::prelude::Entity::PLACEHOLDER,
			previous_transform: &beside,
			current_transform: &beside,
			bounds: &bounds,
		};
		let far_ref = LodRef {
			entity: bevy::prelude::Entity::PLACEHOLDER,
			previous_transform: &far,
			current_transform: &far,
			bounds: &bounds,
		};
		assert_eq!(tower.scene_lod_level(&beside_ref), LodSceneLevel::High);
		assert_eq!(tower.scene_lod_level(&far_ref), LodSceneLevel::Low);
		let local_beside = Transform::from_xyz(0.0, 1.5, 0.0);
		assert_eq!(tower.level_for(&local_beside), LodSceneLevel::High);
		assert_eq!(
			tower.level_for_lod_ref(&far_ref),
			LodSceneLevel::Low,
			"local-space helper still reads the world camera as local"
		);
		let local_ref = LodRef {
			entity: bevy::prelude::Entity::PLACEHOLDER,
			previous_transform: &local_beside,
			current_transform: &local_beside,
			bounds: &bounds,
		};
		assert_eq!(tower.band_progress_for_lod_ref(&local_ref).0, LodSceneLevel::High);
	}

	#[test]
	fn deck_stays_with_the_ring_wall_at_medium() {
		let tower = tower();
		let medium = tower.floor_nodes_for_level(LodSceneLevel::Medium).flatten();
		let high = tower.floor_nodes_for_level(LodSceneLevel::High).flatten();
		assert!(!medium.is_empty());
		assert!(medium
			.iter()
			.all(|node| { node.confines == building_components::ParentConfines::External }));
		assert!(high.len() > medium.len());
		assert!(tower.floor_nodes_for_level(LodSceneLevel::Low).is_empty());
	}
}
