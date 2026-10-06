//! Durham terrain model: SDF composition, LOD generation, Avian index, render.

pub mod base_noise;
pub mod cell;
pub mod collider;
pub mod config;
pub mod geography;
pub mod host;
pub mod index;
pub mod layer;
pub mod plugin;
pub mod presentation;
pub mod render;
pub mod sdf;
pub mod stamp_modulation;
pub mod stamps;
pub mod stream_lod;
pub mod watersheds;

use crate::terrain::render::cascade_chunk_for_cell;
use crate::terrain::stamps::StampLeaf;
use crate::terrain::watersheds::{
	HydroComplexCell, WatershedAproningCell, WatershedCarvingCell, WatershedRimmingCell,
};
use bevy::ecs::template::template;
use bevy::math::bounding::Aabb3d;
use bevy::prelude::*;
use bevy::scene::prelude::{bsn, template_value, Scene};
use lod::gen::{
	GeneratingSpatialIndex, GenerationScheme, Id, LodScene, LodSceneLevel, LodSceneStatus,
	OriginalId, SpatialIndex,
};
use lod::lod_ref::LodRef;
use render_item::mesh::handle::Cached;
use render_item::sdf::cpu_shot::{CpuShotBuilder, WallFaces};
use std::sync::Arc;
use terrain_shaders::TerrainShader;
use terrain_stamps::StampModulation;
use terrain_watersheds::WaterFill;

pub use base_noise::BaseTerrainNoise;
pub use cell::{
	CellTiling, MacroCellLayout, OuterCellRing, TerrainCellLayout, TerrainCellRing,
	MACRO_CELL_SIZE, TERRAIN_CELL_SIZE,
};
pub use chunk::cascade::CascadeChunk;
pub use collider::{
	terrain_collider_covers_xz, TerrainColliderEpoch, TerrainColliderMeshSource,
	TerrainColliderSystems, TerrainFrictionConfig, TerrainSuperseded, TerrainTrimeshCollider,
	TERRAIN_FRICTION,
};
pub use config::TerrainConfig;
pub use geography::{
	GeographicBand, GeographicFamily, GeographicFeature, GeographicFeatureId, GeographicFeatureKind,
};
pub use host::{
	fine_patch_cell_layout, playable_world_cell_layout, produce_terrain_window,
	retarget_presentation_assets, Durham, DurhamCells, TerrainCoverage, TerrainFillSystems,
	TerrainLayoutPinned, TerrainPresentPending, TerrainPresentationDirty, TerrainRetarget,
	TerrainWindow, WorldBaseTerrain, WORLD_FINE_HALF_EXTENT_CELLS, WORLD_OUTER_2X_ROWS,
	WORLD_OUTER_4X_ROWS,
};
pub use index::{
	register_durham_nodes, DurhamNodes, DurhamRoots, TerrainHeightSnapshot, TerrainStorage,
	WaterSurfaceSnapshot,
};
pub use layer::{DurhamHeightSnapshot, DurhamRead, DurhamTerrainConfig};
pub use plugin::{register_terrain_plugin, TerrainResourcesPlugin};
pub use presentation::{
	sync_visual_terrain_host_pose, PresentedTerrainScene, TerrainBackground,
	TerrainBackgroundRegionPresenter, TerrainFar, TerrainFarRegionPresenter, TerrainMeshLodBand,
	TerrainNear, TerrainNearRegionPresenter, TerrainPresentationAssets, TerrainPresenterState,
	TerrainRegionPresenter, TerrainStreamMarker, TerrainStreamPresenterState,
	TerrainStreamRegionPresenter, TerrainVisualHost,
};
pub use render::TerrainRenderItem;
pub use sdf::{ComposedTerrain, ElevationModulation, TerrainSdf};
pub use stamp_modulation::ComposedElevationOp;
pub use stamps::{
	CanyonHighPassControllerCell, CanyonHighPassControllerLayout, CanyonHighPassStampCell,
	CanyonLowPassControllerCell, CanyonLowPassControllerLayout, CanyonLowPassStampCell,
	MassifHighPassControllerCell, MassifHighPassControllerLayout, MassifHighPassStampCell,
	MassifLowPassControllerCell, MassifLowPassControllerLayout, MassifLowPassStampCell,
	PlateauControllerLayout, PlateauHighPassControllerCell, PlateauHighPassControllerLayout,
	PlateauHighPassStampCell, PlateauLowPassControllerCell, PlateauLowPassControllerLayout,
	PlateauLowPassStampCell, PocketWaterHighPassControllerCell,
	PocketWaterHighPassControllerLayout, PocketWaterHighPassStampCell,
	PocketWaterLowPassControllerCell, PocketWaterLowPassControllerLayout,
	PocketWaterLowPassStampCell, RollingHighPassControllerCell, RollingHighPassControllerLayout,
	RollingHighPassStampCell, RollingLowPassControllerCell, RollingLowPassControllerLayout,
	RollingLowPassStampCell, TerrainStampConfigs, ValleyHighPassControllerCell,
	ValleyHighPassControllerLayout, ValleyHighPassStampCell, ValleyLowPassControllerCell,
	ValleyLowPassControllerLayout, ValleyLowPassStampCell,
};
pub use stamps::{
	CanyonLowPassStampCell as CanyonStampCell, MassifLowPassStampCell as MassifStampCell,
	PlateauLowPassStampCell as PlateauStampCell,
	PocketWaterLowPassStampCell as PocketWaterStampCell,
	RollingLowPassStampCell as RollingStampCell, ValleyLowPassStampCell as ValleyStampCell,
};
pub use stream_lod::{
	stream_banded_draws, stream_banded_level, stream_banded_scene, StreamBandedLod,
};
pub use watersheds::{
	PocketHighPassCell, PocketLowPassCell, PocketWater, PocketWatersHighPass, PocketWatersLowPass,
	PrePocketHighPassCell, PrePocketHighPassLayout, PrePocketLowPassCell, PrePocketLowPassLayout,
	WatershedBandPass, WatershedConfigs, WatershedLeafBounds, WatershedLeafKind,
};
/// Low-pass aliases kept for older HUD / call sites.
pub use watersheds::{
	PocketLowPassCell as PocketCell, PocketWatersLowPass as LakeStampCell,
	PrePocketLowPassCell as PrePocketCell, PrePocketLowPassLayout as PrePocketLayout,
};

/// CpuShot wrapper stored on [`Terrain`] and used by Durham fill + overlay presenters.
pub type TerrainMeshBuilder = CpuShotBuilder<Arc<ComposedTerrain>>;

/// Stamp-composed terrain **before** watershed stamps.
#[derive(Debug, Clone, Component)]
pub struct PreWatershedTerrain {
	pub cell: Aabb3d,
	pub base: BaseTerrainNoise,
	pub modulations: Vec<StampModulation>,
	pub jersey_leaves: Vec<Aabb3d>,
	pub sdf: ComposedTerrain,
}

impl PreWatershedTerrain {
	pub fn compose_sdf(
		base: &BaseTerrainNoise,
		modulations: &[StampModulation],
	) -> ComposedTerrain {
		let mut sdf = base.sdf.clone();
		for modulation in modulations {
			sdf.add_elevation_modulation(Box::new(modulation.clone()));
		}
		ComposedTerrain::from_terrain(sdf)
	}

	/// Pre-watershed height at `(x, z)`, materializing the fine origin cell under it.
	pub fn sample_height<S>(spatial_index: &mut S, x: f32, z: f32) -> Option<f32>
	where
		S: GeneratingSpatialIndex<Self> + GeneratingSpatialIndex<TerrainCellLayout>,
	{
		let layout = GeneratingSpatialIndex::<TerrainCellLayout>::get_one_or_generate(
			spatial_index,
			Id::Universal,
		)?;
		let id = Id::from_cell(layout.fine_cell_bounds_containing(x, z));
		let pre = GeneratingSpatialIndex::<Self>::get_one_or_generate(spatial_index, id)?;
		Some(pre.sdf.terrain().height_at_with_all_modulations(x, z))
	}
}

/// Final terrain cell: pre-watershed heightfield + Watershed lake stamps.
#[derive(Debug, Clone, Component)]
pub struct Terrain {
	pub cell: Aabb3d,
	pub base: BaseTerrainNoise,
	/// Flattened jersey + marazion leaf elevation ops.
	pub modulations: Vec<ComposedElevationOp>,
	/// Leaf AABBs whose jersey stamps contributed (debug / HUD).
	pub jersey_leaves: Vec<Aabb3d>,
	/// Leaf AABBs whose Watershed lake stamps contributed (plus empties for debug).
	pub marazion_leaves: Vec<WatershedLeafBounds>,
	/// Fills from this origin cell's cellular [`HydroComplexCell`].
	///
	/// ComplexCell unions hydrology nodes from intersecting authored leaves
	/// (all bands); fills share that union φ. Collected with carve → rim →
	/// apron before SDF compose so [`crate::water::Water`] can evaluate wet
	/// volume against the finished heightfield.
	pub marazion_fills: Vec<WaterFill>,
	pub sdf: Arc<ComposedTerrain>,
	pub material: Handle<TerrainShader>,
	pub res_2: u8,
	/// Moving presentation band for streamed near / far / background terrain.
	pub stream_ring: Option<TerrainCellRing>,
	/// Per-face CpuShot edge height walls (LOD seam skirts).
	pub wall_faces: WallFaces,
}

impl Terrain {
	pub fn compose_sdf(
		base: &BaseTerrainNoise,
		modulations: &[ComposedElevationOp],
	) -> ComposedTerrain {
		let mut sdf = base.sdf.clone();
		for modulation in modulations {
			sdf.add_elevation_modulation(Box::new(modulation.clone()));
		}
		ComposedTerrain::from_terrain(sdf)
	}

	/// Shared CpuShot identity for fill [`Cached`] dispatch and overlay chunk refs.
	pub fn mesh_builder(&self) -> TerrainMeshBuilder {
		CpuShotBuilder::new(Arc::clone(&self.sdf)).with_wall_faces(self.wall_faces)
	}

	/// World pose for CpuShot verts, which are local to the cascade origin.
	pub fn chunk_pose(&self) -> Transform {
		Transform::from_translation(cascade_chunk_for_cell(self.cell, self.res_2).origin)
	}

	/// Near-ring (or unbanded FinePatch) cells carry a trimesh on this scene.
	pub fn seeds_collision(&self) -> bool {
		self.stream_ring.map(|ring| ring.seeds_collision()).unwrap_or(true)
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

	pub fn scene(&self) -> impl Scene + 'static {
		let chunk = cascade_chunk_for_cell(self.cell, self.res_2);
		let transform = self.chunk_pose();
		let builder = self.mesh_builder();
		let material = self.material.clone();
		bsn! {
			template_value(transform)
			template_value(chunk)
			template(move |_ctx| Ok(Cached::new(builder.clone())))
			MeshMaterial3d::<TerrainShader>({material.clone()})
			TerrainColliderMeshSource
		}
	}

	fn center(&self) -> Vec3 {
		(Vec3::from(self.cell.min) + Vec3::from(self.cell.max)) * 0.5
	}

	fn mesh_scene(&self) -> impl Scene + 'static {
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

impl crate::terrain::stream_lod::StreamBandedLod for Terrain {
	fn stream_ring(&self) -> Option<TerrainCellRing> {
		self.stream_ring
	}

	fn stream_center(&self) -> Vec3 {
		self.center()
	}
}

impl LodScene for Terrain {
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
		stream_banded_scene(self, level, || self.mesh_scene())
	}
}

/// Non-empty jersey stamp leaves on one origin cell, in pull order.
#[derive(Default)]
struct JerseyStamps {
	modulations: Vec<StampModulation>,
	leaves: Vec<Aabb3d>,
}

impl JerseyStamps {
	fn pull<T: StampLeaf, S: GeneratingSpatialIndex<T>>(
		&mut self,
		spatial_index: &mut S,
		bounds: Aabb3d,
	) -> Option<()> {
		GeneratingSpatialIndex::<T>::for_each_origin(spatial_index, bounds, |stamp| {
			if !stamp.modulations().is_empty() {
				self.leaves.push(stamp.cell());
				self.modulations.extend_from_slice(stamp.modulations());
			}
		})
	}
}

/// Pre-watershed: base noise + jersey landform stamps (including pocket-water height).
///
/// Origin-grid root: tiles [`TerrainCellLayout`] directly. Each stamp band is
/// one leaf bound; its controller grid and configs are that band's concern.
impl<S> GenerationScheme<S> for PreWatershedTerrain
where
	S: GeneratingSpatialIndex<TerrainCellLayout>
		+ GeneratingSpatialIndex<BaseTerrainNoise>
		+ GeneratingSpatialIndex<PlateauHighPassStampCell>
		+ GeneratingSpatialIndex<MassifHighPassStampCell>
		+ GeneratingSpatialIndex<CanyonHighPassStampCell>
		+ GeneratingSpatialIndex<PocketWaterHighPassStampCell>
		+ GeneratingSpatialIndex<RollingHighPassStampCell>
		+ GeneratingSpatialIndex<ValleyHighPassStampCell>
		+ GeneratingSpatialIndex<PlateauLowPassStampCell>
		+ GeneratingSpatialIndex<MassifLowPassStampCell>
		+ GeneratingSpatialIndex<CanyonLowPassStampCell>
		+ GeneratingSpatialIndex<PocketWaterLowPassStampCell>
		+ GeneratingSpatialIndex<RollingLowPassStampCell>
		+ GeneratingSpatialIndex<ValleyLowPassStampCell>,
{
	fn original_ids_for(spatial_index: &mut S, region: Aabb3d) -> Vec<OriginalId> {
		TerrainCellLayout::original_cell_ids_for(spatial_index, region)
	}

	fn build_with_id(spatial_index: &mut S, id: Id) -> Option<(Self, Aabb3d)> {
		let bounds = id.origin_cell_bounds()?;
		let base = GeneratingSpatialIndex::<BaseTerrainNoise>::get_one_or_generate(
			spatial_index,
			Id::Universal,
		)?
		.clone();

		// Composition order is global: high-pass (regional) bands, then low-pass (detail).
		let mut stamps = JerseyStamps::default();
		stamps.pull::<PlateauHighPassStampCell, _>(spatial_index, bounds)?;
		stamps.pull::<MassifHighPassStampCell, _>(spatial_index, bounds)?;
		stamps.pull::<CanyonHighPassStampCell, _>(spatial_index, bounds)?;
		stamps.pull::<PocketWaterHighPassStampCell, _>(spatial_index, bounds)?;
		stamps.pull::<RollingHighPassStampCell, _>(spatial_index, bounds)?;
		stamps.pull::<ValleyHighPassStampCell, _>(spatial_index, bounds)?;
		stamps.pull::<PlateauLowPassStampCell, _>(spatial_index, bounds)?;
		stamps.pull::<MassifLowPassStampCell, _>(spatial_index, bounds)?;
		stamps.pull::<CanyonLowPassStampCell, _>(spatial_index, bounds)?;
		stamps.pull::<PocketWaterLowPassStampCell, _>(spatial_index, bounds)?;
		stamps.pull::<RollingLowPassStampCell, _>(spatial_index, bounds)?;
		stamps.pull::<ValleyLowPassStampCell, _>(spatial_index, bounds)?;

		let JerseyStamps { modulations, leaves: jersey_leaves } = stamps;
		let sdf = Self::compose_sdf(&base, &modulations);
		Some((Self { cell: bounds, base, modulations, jersey_leaves, sdf }, bounds))
	}
}

/// Final terrain: pre-watershed + Watershed correction stages (carve → rim → apron).
///
/// Shares [`PreWatershedTerrain`]'s origin ids. Pocket-water leaves, the
/// hydro complex, and stage cells are bound only as the leaves this scheme
/// reads; their pocket / pre-pocket / config stacks resolve at the index.
impl<S> GenerationScheme<S> for Terrain
where
	S: GeneratingSpatialIndex<PreWatershedTerrain>
		+ GeneratingSpatialIndex<PocketWatersHighPass>
		+ GeneratingSpatialIndex<PocketWatersLowPass>
		+ GeneratingSpatialIndex<HydroComplexCell>
		+ GeneratingSpatialIndex<WatershedCarvingCell>
		+ GeneratingSpatialIndex<WatershedRimmingCell>
		+ GeneratingSpatialIndex<WatershedAproningCell>
		+ GeneratingSpatialIndex<TerrainPresentationAssets>
		+ SpatialIndex<TerrainCellLayout>,
{
	fn original_ids_for(spatial_index: &mut S, region: Aabb3d) -> Vec<OriginalId> {
		GeneratingSpatialIndex::<PreWatershedTerrain>::original_ids_for(spatial_index, region)
	}

	fn build_with_id(spatial_index: &mut S, id: Id) -> Option<(Self, Aabb3d)> {
		let bounds = id.origin_cell_bounds()?;
		let pre =
			GeneratingSpatialIndex::<PreWatershedTerrain>::get_one_or_generate(spatial_index, id)?
				.clone();

		// Authored leaf overlays (banded); hydrology composition is cellular below.
		let mut marazion_leaves = Vec::new();
		GeneratingSpatialIndex::<PocketWatersHighPass>::for_each_origin(
			spatial_index,
			bounds,
			|leaf| marazion_leaves.push(leaf.leaf_bounds()),
		)?;
		GeneratingSpatialIndex::<PocketWatersLowPass>::for_each_origin(
			spatial_index,
			bounds,
			|leaf| marazion_leaves.push(leaf.leaf_bounds()),
		)?;

		let complex =
			GeneratingSpatialIndex::<HydroComplexCell>::get_one_or_generate(spatial_index, id)?
				.indexed()
				.cloned();

		// Keep stage cells materialized for later policy work; elevation uses
		// the cellular HydroComplex directly (internal carve → rim → apron).
		GeneratingSpatialIndex::<WatershedCarvingCell>::get_or_generate(spatial_index, id)?;
		GeneratingSpatialIndex::<WatershedRimmingCell>::get_or_generate(spatial_index, id)?;
		GeneratingSpatialIndex::<WatershedAproningCell>::get_or_generate(spatial_index, id)?;

		let marazion_fills = complex.iter().cloned().map(WaterFill::from_hydro).collect();
		let modulations: Vec<_> = pre
			.modulations
			.into_iter()
			.map(ComposedElevationOp::Stamp)
			.chain(complex.map(ComposedElevationOp::Hydro))
			.collect();
		let sdf = Arc::new(Self::compose_sdf(&pre.base, &modulations));

		let layout = SpatialIndex::<TerrainCellLayout>::get(spatial_index, Id::Universal)
			.cloned()
			.unwrap_or_default();
		let assets = GeneratingSpatialIndex::<TerrainPresentationAssets>::get_one_or_generate(
			spatial_index,
			Id::Universal,
		)?;
		let (res_2, wall_faces) = assets.mesh_params_for_cell(bounds, &layout);
		let cell_size = (Vec3::from(bounds.max) - Vec3::from(bounds.min)).x;

		Some((
			Self {
				cell: bounds,
				base: pre.base,
				modulations,
				jersey_leaves: pre.jersey_leaves,
				marazion_leaves,
				marazion_fills,
				sdf,
				material: assets.material.clone(),
				res_2,
				stream_ring: layout.stream_ring_for_cell_size(cell_size),
				wall_faces,
			},
			bounds,
		))
	}
}
