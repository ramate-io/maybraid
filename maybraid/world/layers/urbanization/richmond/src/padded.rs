//! Terrain origin cell with development-pad elevation ops applied.

use bevy::ecs::template::template;
use bevy::math::bounding::Aabb3d;
use bevy::math::Vec3;
use bevy::prelude::*;
use bevy::scene::prelude::{bsn, template_value, Scene};
use durham::terrain::ElevationModulation;
use durham::{
	cascade_chunk_for_cell, stream_banded_level, stream_banded_scene, ComposedTerrain,
	StreamBandedLod, Terrain, TerrainCellRing, TerrainColliderMeshSource, TerrainMeshBuilder,
	TerrainSdf,
};
use lod::gen::{GenerationScheme, Id, LodScene, LodSceneLevel, LodSceneStatus, OriginalId};
use lod::hcsg::shared::{self, GenerationContext};
use lod::hcsg::HcsgStorage;
use lod::lod_ref::LodRef;
use render_item::mesh::handle::Cached;
use render_item::sdf::cpu_shot::{CpuShotBuilder, WallFaces};
use std::marker::PhantomData;
use std::sync::Arc;
use terrain_layer_model::TerrainCell;
use terrain_shaders::TerrainShader;

use crate::compose::PadComposable;
use crate::developments::RichmondDevelopment;
use crate::ground::RichmondGround;
use crate::pad::PadComplex;
use crate::storage::column_bounds;

/// Durham [`Terrain`] plus overlapping development pads.
#[derive(Debug, Clone, Component)]
pub struct TerrainWithPads {
	pub cell: Aabb3d,
	pub sdf: Arc<ComposedTerrain>,
	pub material: Handle<TerrainShader>,
	pub res_2: u8,
	pub wall_faces: WallFaces,
	pub pad_count: usize,
	/// Moving stream band copied from the source Durham cell.
	pub stream_ring: Option<TerrainCellRing>,
}

impl TerrainWithPads {
	/// Without pad nodes the terrain is wrapped as is: its surface and walls.
	pub fn compose<'a>(terrain: &Terrain, pads: impl IntoIterator<Item = &'a PadComplex>) -> Self {
		let mut pad_count = 0;
		let mut nodes = Vec::new();
		for pad in pads {
			pad_count += 1;
			nodes.extend(pad.pads.iter().cloned());
		}
		let merged = PadComplex::from_nodes(nodes);
		if merged.is_empty() {
			return Self {
				cell: terrain.cell,
				sdf: Arc::clone(&terrain.sdf),
				material: terrain.material.clone(),
				res_2: terrain.res_2,
				wall_faces: terrain.wall_faces,
				pad_count: 0,
				stream_ring: terrain.stream_ring,
			};
		}
		let mut sdf: TerrainSdf = terrain.sdf.terrain().clone();
		sdf.add_elevation_modulation(Box::new(merged) as Box<dyn ElevationModulation>);
		Self {
			cell: terrain.cell,
			sdf: Arc::new(ComposedTerrain::from_terrain(sdf)),
			material: terrain.material.clone(),
			res_2: terrain.res_2,
			// Building-skirt pads can still meet origin-cell faces on a large
			// footprint; interior skirts close the CpuShot crack.
			wall_faces: WallFaces::ALL,
			pad_count,
			stream_ring: terrain.stream_ring,
		}
	}

	pub fn mesh_builder(&self) -> TerrainMeshBuilder {
		CpuShotBuilder::new(Arc::clone(&self.sdf)).with_wall_faces(self.wall_faces)
	}

	/// World pose for CpuShot verts, which are local to the cascade origin.
	pub fn chunk_pose(&self) -> Transform {
		Transform::from_translation(cascade_chunk_for_cell(self.cell, self.res_2).origin)
	}

	/// Posed fill entity. [`Mesh3d`] (and the trimesh, when [`Self::seeds_collision`])
	/// land on this same entity after Cached fulfill.
	pub fn spawn_fill(
		&self,
		commands: &mut Commands,
		visibility: Visibility,
		collide: bool,
	) -> Entity {
		let chunk = cascade_chunk_for_cell(self.cell, self.res_2);
		let entity = commands
			.spawn((
				self.chunk_pose(),
				chunk,
				Cached::new(self.mesh_builder()),
				MeshMaterial3d(self.material.clone()),
				visibility,
			))
			.id();
		if collide {
			commands.entity(entity).insert(TerrainColliderMeshSource);
		}
		entity
	}

	/// Visual fill plus the trimesh source.
	pub fn scene(&self) -> impl Scene + 'static {
		(self.mesh_scene(), bsn! { TerrainColliderMeshSource })
	}

	pub fn seeds_collision(&self) -> bool {
		self.stream_ring.map(|ring| ring.seeds_collision()).unwrap_or(true)
	}

	fn center(&self) -> Vec3 {
		(Vec3::from(self.cell.min) + Vec3::from(self.cell.max)) * 0.5
	}

	/// Visual fill. Pose is the cascade origin; [`Mesh3d`] fulfills onto this root.
	pub fn mesh_scene(&self) -> impl Scene + 'static {
		let chunk = cascade_chunk_for_cell(self.cell, self.res_2);
		let transform = self.chunk_pose();
		let builder = self.mesh_builder();
		let material = self.material.clone();
		bsn! {
			template_value(transform)
			template_value(chunk)
			template(move |_ctx| Ok(Cached::new(builder.clone())))
			MeshMaterial3d::<TerrainShader>({material.clone()})
		}
	}
}

impl TerrainCell for TerrainWithPads {
	type Mesh = TerrainMeshBuilder;

	fn bounds(&self) -> Aabb3d {
		self.cell
	}

	fn mesh_builder(&self) -> TerrainMeshBuilder {
		TerrainWithPads::mesh_builder(self)
	}

	fn chunk_pose(&self) -> Transform {
		TerrainWithPads::chunk_pose(self)
	}

	fn seeds_collision(&self) -> bool {
		TerrainWithPads::seeds_collision(self)
	}

	fn res_2(&self) -> u8 {
		self.res_2
	}
}

impl StreamBandedLod for TerrainWithPads {
	fn stream_ring(&self) -> Option<TerrainCellRing> {
		self.stream_ring
	}

	fn stream_center(&self) -> Vec3 {
		self.center()
	}
}

impl LodScene for TerrainWithPads {
	fn scene_lod_level(&self, lod_ref: &LodRef) -> LodSceneLevel {
		stream_banded_level(self, lod_ref.current_transform)
	}

	fn scene_lod_status(&self, lod_ref: &LodRef) -> LodSceneStatus {
		let previous = stream_banded_level(self, lod_ref.previous_transform);
		let current = stream_banded_level(self, lod_ref.current_transform);
		if previous == current {
			LodSceneStatus::Unchanged
		} else {
			LodSceneStatus::Changed(current)
		}
	}

	fn scene_with_level(&self, _lod_ref: &LodRef, level: LodSceneLevel) -> impl Scene + 'static {
		if self.seeds_collision() {
			stream_banded_scene(self, level, || self.scene())
		} else {
			stream_banded_scene(self, level, || self.mesh_scene())
		}
	}
}

/// A cell of ground `G` with the pads of the developments over it composed in.
pub struct PaddedTerrain<G> {
	pub surface: TerrainWithPads,
	_ground: PhantomData<fn() -> G>,
}

impl<G: RichmondGround> LodScene for PaddedTerrain<G> {
	fn scene_lod_level(&self, lod_ref: &LodRef) -> LodSceneLevel {
		self.surface.scene_lod_level(lod_ref)
	}

	fn scene_lod_status(&self, lod_ref: &LodRef) -> LodSceneStatus {
		self.surface.scene_lod_status(lod_ref)
	}

	fn scene_with_level(&self, lod_ref: &LodRef, level: LodSceneLevel) -> impl Scene + 'static {
		self.surface.scene_with_level(lod_ref, level)
	}
}

impl<G> PaddedTerrain<G> {
	pub fn new(surface: TerrainWithPads) -> Self {
		Self { surface, _ground: PhantomData }
	}
}

impl<G: RichmondGround> PaddedTerrain<G> {
	/// Stored padded surface with the greatest XZ overlap with `region`; the
	/// finest on a tie.
	pub fn best_overlapping(storage: &HcsgStorage, region: Aabb3d) -> Option<&TerrainWithPads> {
		let mut best: Option<(f32, f32, &TerrainWithPads)> = None;
		for id in storage.overlapping::<Self>(column_bounds(region)) {
			let Some(entry) = storage.entry::<Self>(id) else {
				continue;
			};
			let overlap_x = (region.max.x.min(entry.bounds.max.x)
				- region.min.x.max(entry.bounds.min.x))
			.max(0.0);
			let overlap_z = (region.max.z.min(entry.bounds.max.z)
				- region.min.z.max(entry.bounds.min.z))
			.max(0.0);
			let overlap = overlap_x * overlap_z;
			if overlap <= 1e-3 {
				continue;
			}
			let span = (entry.bounds.max.x - entry.bounds.min.x)
				.max(entry.bounds.max.z - entry.bounds.min.z);
			if best.is_none_or(|(best_overlap, best_span, _)| {
				overlap > best_overlap || (overlap == best_overlap && span < best_span)
			}) {
				best = Some((overlap, span, &entry.value.surface));
			}
		}
		best.map(|(_, _, terrain)| terrain)
	}
}

impl<G: RichmondGround> GenerationScheme<HcsgStorage> for PaddedTerrain<G> {
	/// The ground's stored cells: its rings tile differently around each
	/// center, so padded cells follow whatever ground is streamed.
	fn original_ids_for(storage: &mut HcsgStorage, region: Aabb3d) -> Vec<OriginalId> {
		storage.overlapping::<G::Cell>(region).into_iter().map(OriginalId).collect()
	}

	/// Generates the developments over the cell. A cell no pad reaches has
	/// no padded surface; the ground presents it as is.
	fn build_with_id(storage: &mut HcsgStorage, id: Id) -> Option<(Self, Aabb3d)> {
		let bounds = storage.entry::<G::Cell>(id)?.bounds;
		for OriginalId(development) in storage.original_ids_for::<RichmondDevelopment<G>>(bounds) {
			storage.get_or_generate::<RichmondDevelopment<G>>(development);
		}
		let pads = RichmondDevelopment::<G>::merged_pads(storage, bounds);
		if pads.is_empty() {
			return None;
		}
		let surface = storage.get::<G::Cell>(id)?.compose_pads(&pads);
		Some((Self::new(surface), bounds))
	}
}

/// Total over the ground: every cell of `G` is presented padded, wrapped as
/// is where no pad reaches it, so the raw cell is never presented beneath.
impl<G: RichmondGround> shared::GenerationScheme for PaddedTerrain<G> {
	fn original_ids_for(cx: &mut GenerationContext, region: Aabb3d) -> Vec<OriginalId> {
		cx.original_ids_for::<G::Cell>(region)
	}

	/// The cell with the pads of every development over it.
	fn build_with_id(cx: &mut GenerationContext, id: Id) -> Option<(Self, Aabb3d)> {
		let cell = cx.get_or_generate::<G::Cell>(id)?;
		let bounds = cell.bounds();
		let developments: Vec<_> = cx
			.original_ids_for::<RichmondDevelopment<G>>(bounds)
			.into_iter()
			.filter_map(|OriginalId(id)| cx.get_or_generate::<RichmondDevelopment<G>>(id))
			.collect();
		let pads = RichmondDevelopment::merge_pads(bounds, developments.iter().map(Arc::as_ref));
		Some((Self::new(cell.compose_pads(&pads)), bounds))
	}
}

/// Marks a spawned padded-terrain scene root.
#[derive(Component, Debug, Clone, Copy)]
pub struct PresentedPaddedTerrainScene(pub Id);

#[cfg(test)]
mod tests {
	use super::*;
	use durham::{stream_banded_draws, stream_banded_level, TerrainSdf, TERRAIN_CELL_SIZE};
	use lod::LodSceneLevel;

	fn far_ring() -> TerrainCellRing {
		TerrainCellRing {
			cell_size: 2.0 * TERRAIN_CELL_SIZE,
			res_2: 4,
			anchor_step: 4.0 * TERRAIN_CELL_SIZE,
			high_inner_radius: 8.0 * TERRAIN_CELL_SIZE,
			high_outer_radius: 16.0 * TERRAIN_CELL_SIZE,
			cull_margin: 8.0 * TERRAIN_CELL_SIZE,
		}
	}

	fn pad_at(center: Vec3, ring: TerrainCellRing) -> TerrainWithPads {
		let half = ring.cell_size * 0.5;
		let bounds = Aabb3d::from_min_max(
			Vec3::new(center.x - half, -1.0, center.z - half),
			Vec3::new(center.x + half, 1.0, center.z + half),
		);
		TerrainWithPads {
			cell: bounds,
			sdf: Arc::new(ComposedTerrain::from_terrain(TerrainSdf::new(1, 20.0))),
			material: Handle::default(),
			res_2: ring.res_2,
			wall_faces: WallFaces::NONE,
			pad_count: 0,
			stream_ring: Some(ring),
		}
	}

	#[test]
	fn far_pad_underfoot_is_empty_high() {
		let pad = pad_at(Vec3::ZERO, far_ring());
		let viewer = Transform::IDENTITY;
		let level = stream_banded_level(&pad, &viewer);
		assert_eq!(level, LodSceneLevel::High);
		assert!(!stream_banded_draws(&pad, level));
	}

	#[test]
	fn far_pad_in_ring_is_drawn_medium() {
		let ring = far_ring();
		let pad = pad_at(Vec3::new(ring.high_inner_radius + 80.0, 0.0, 0.0), ring);
		let viewer = Transform::IDENTITY;
		let level = stream_banded_level(&pad, &viewer);
		assert_eq!(level, LodSceneLevel::Medium);
		assert!(stream_banded_draws(&pad, level));
	}

	#[test]
	fn unbanded_pad_always_draws_high() {
		let mut pad = pad_at(Vec3::ZERO, far_ring());
		pad.stream_ring = None;
		let viewer = Transform::IDENTITY;
		assert_eq!(stream_banded_level(&pad, &viewer), LodSceneLevel::High);
		assert!(stream_banded_draws(&pad, LodSceneLevel::High));
	}

	#[test]
	fn chunk_pose_is_cascade_origin() {
		let pad = pad_at(Vec3::new(160.0, 0.0, -320.0), far_ring());
		let chunk = cascade_chunk_for_cell(pad.cell, pad.res_2);
		assert_eq!(pad.chunk_pose(), Transform::from_translation(chunk.origin));
		assert_ne!(pad.chunk_pose(), Transform::IDENTITY);
	}
}
