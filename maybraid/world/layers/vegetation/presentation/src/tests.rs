use std::collections::HashSet;

use bevy::ecs::system::{SystemParam, SystemState};
use bevy::math::bounding::Aabb3d;
use bevy::math::{Vec2, Vec3};
use bevy::prelude::{App, MinimalPlugins, NextState, Plugin, World};
use bevy::state::app::StatesPlugin;
use chico::{ChicoGrove, ForestIndex, ForestLodChan, ForestPresenterState};
use vegetation_groves::{GroveHeightModulation, GroveTerrain, GroveWorldSample, ModulatedGroveSample};
use durham::{
	BaseTerrainNoise, Durham, TerrainCellLayout, TerrainConfig, TerrainEntryStore,
	TerrainHeightSnapshot, WorldBaseTerrain, TERRAIN_CELL_SIZE,
};
use lod::gen::{Id, Version};
use lod::lod_ref::LodRef;
use lod::presentation::RegionPresenter;
use lod::{LodPresentCullPlugin, LodPresentGate, LodPresentKeepRegion, LodPresentPlugin};
use richmond::{DevelopmentCell, DevelopmentConfig, DevelopmentEntryStore, PadComplex};
use urbanization_cells::UrbanizationIndex;
use terrain_layer_model::{HeightField, OnTerrain, TerrainModel, TerrainView};
use layer_stack::{
	install_lod_present_gate, subscribe_mode, ActiveGenerationMode, GenerationMode,
	GenerationModePlugin, ModeSubscribers, ModeSubscription,
};
use urbanization_layer_model::Urbanization;

use crate::{GroundGroveSample, VegetationPresent, VegetationPresentationPlugin};

type Urbanized = Urbanization<richmond::Richmond<OnTerrain<Durham>>>;

/// Pre-move `OwnedDurhamTerrain`: stored height, else base noise.
struct OwnedDurham {
	snapshot: TerrainHeightSnapshot,
	layout: TerrainCellLayout,
	fallback: BaseTerrainNoise,
}

impl GroveTerrain for OwnedDurham {
	fn height_at(&self, position: Vec3) -> f32 {
		self.snapshot
			.composed_height_at(&self.layout, position.x, position.z)
			.unwrap_or_else(|| self.fallback.height_at(position.x, position.z))
	}
}

/// Pre-move `DurhamGroveSample`.
struct DurhamGroveSample<T>(T);

impl<T: GroveTerrain> GroveWorldSample for DurhamGroveSample<T> {
	fn height_at(&self, position: Vec3) -> f32 {
		self.0.height_at(position)
	}

	fn steepness_at(&self, position: Vec3) -> f32 {
		self.0.steepness_at(position)
	}
}

/// Pre-move `DevelopmentPadModulation`.
struct DevelopmentPadModulation(PadComplex);

impl GroveHeightModulation for DevelopmentPadModulation {
	fn modulate_height(&self, base_height: f32, x: f32, z: f32) -> f32 {
		self.0.modify_elevation(base_height, x, z)
	}
}

fn assert_same(
	old: &impl GroveWorldSample,
	new: &impl GroveWorldSample,
	xz: Vec2,
) -> anyhow::Result<()> {
	let position = Vec3::new(xz.x, 0.0, xz.y);
	let old_height = old.height_at(position);
	let new_height = new.height_at(position);
	anyhow::ensure!(
		old_height == new_height,
		"height at {xz:?}: {old_height} vs {new_height}"
	);
	let old_steep = old.steepness_at(position);
	let new_steep = new.steepness_at(position);
	anyhow::ensure!(
		old_steep == new_steep,
		"steepness at {xz:?}: {old_steep} vs {new_steep}"
	);
	Ok(())
}

#[test]
fn ground_grove_sample_matches_durham_and_modulated_samples() -> anyhow::Result<()> {
	let mut world = World::new();
	let layout = TerrainCellLayout::default();
	let base = BaseTerrainNoise::from_config(&TerrainConfig::new(42));
	world.insert_resource(TerrainEntryStore::default());
	world.insert_resource(layout.clone());
	world.insert_resource(WorldBaseTerrain(base.clone()));
	world.insert_resource(DevelopmentEntryStore::default());
	world.insert_resource(UrbanizationIndex::default());
	world
		.resource_mut::<TerrainEntryStore>()
		.insert_base_terrain_for_test(&layout, 0, 0, base.clone());

	let store = world.resource::<TerrainEntryStore>();
	let probe = Aabb3d::from_min_max(Vec3::new(1.0, -1_000.0, 1.0), Vec3::new(2.0, 1_000.0, 2.0));
	let cell_id = store
		.terrain_ids_overlapping(probe)
		.into_iter()
		.next()
		.ok_or_else(|| anyhow::anyhow!("stored cell"))?;
	let region = store
		.terrain(cell_id)
		.map(terrain_layer_model::TerrainCell::bounds)
		.ok_or_else(|| anyhow::anyhow!("cell bounds"))?;
	let config = DevelopmentConfig::from_world_seed(42);
	world.resource_mut::<DevelopmentEntryStore>().insert_cell(
		Id::from_cell(region),
		DevelopmentCell::with_les_halles(region, 12.0, &config),
	);

	let owned = OwnedDurham {
		snapshot: world.resource::<TerrainEntryStore>().height_snapshot(),
		layout: layout.clone(),
		fallback: base,
	};
	let pads = world.resource::<DevelopmentEntryStore>().merged_pad_complex(region);
	let old_durham = DurhamGroveSample(owned.clone());
	let old_urban = ModulatedGroveSample::new(
		DurhamGroveSample(owned),
		vec![DevelopmentPadModulation(pads)],
	);

	let durham_snapshot = {
		let mut state = SystemState::<TerrainView<OnTerrain<Durham>>>::new(&mut world);
		let view = state.get(&world)?;
		view.snapshot(region)
	};
	let urban_snapshot = {
		let mut state = SystemState::<TerrainView<Urbanized>>::new(&mut world);
		let view = state.get(&world)?;
		view.snapshot(region)
	};
	let new_durham = GroundGroveSample::new(durham_snapshot.clone());
	let new_urban = GroundGroveSample::new(urban_snapshot);

	let center = Vec2::new(
		(region.min.x + region.max.x) * 0.5,
		(region.min.z + region.max.z) * 0.5,
	);
	let ungenerated = Vec2::new(5_000.0, -4_000.0);
	let center_pos = Vec3::new(center.x, 0.0, center.y);
	let center_mod = old_urban.height_at(center_pos);
	let center_raw = old_durham.height_at(center_pos);
	let skirt = skirt_point(&old_urban, &old_durham, region).map_err(|_| {
		anyhow::anyhow!("no skirt sample; center modulated {center_mod} raw {center_raw}")
	})?;
	let outside = outside_point(&old_urban, &old_durham, &durham_snapshot, center, region)?;

	for xz in [center, skirt, outside, ungenerated] {
		assert_same(&old_durham, &new_durham, xz)?;
		assert_same(&old_urban, &new_urban, xz)?;
	}
	anyhow::ensure!(
		durham_snapshot.height_at(ungenerated).is_none(),
		"ungenerated point should miss the stored cell"
	);
	Ok(())
}

fn skirt_point(
	urban: &impl GroveWorldSample,
	durham: &impl GroveWorldSample,
	region: Aabb3d,
) -> anyhow::Result<Vec2> {
	let mut x = region.min.x + 0.5;
	while x < region.max.x {
		let mut z = region.min.z + 0.5;
		while z < region.max.z {
			let xz = Vec2::new(x, z);
			let position = Vec3::new(xz.x, 0.0, xz.y);
			let modulated = urban.height_at(position);
			let raw = durham.height_at(position);
			if modulated != raw && (modulated - 12.0).abs() > 1e-3 {
				return Ok(xz);
			}
			z += 2.0;
		}
		x += 2.0;
	}
	Err(anyhow::anyhow!("no skirt sample"))
}

fn outside_point(
	urban: &impl GroveWorldSample,
	durham: &impl GroveWorldSample,
	snapshot: &impl HeightField,
	center: Vec2,
	region: Aabb3d,
) -> anyhow::Result<Vec2> {
	let mut x = region.min.x + 0.5;
	while x < region.max.x {
		let xz = Vec2::new(x, region.min.z + 0.5);
		let position = Vec3::new(xz.x, 0.0, xz.y);
		if snapshot.height_at(xz).is_some()
			&& urban.height_at(position) == durham.height_at(position)
			&& xz.distance(center) > 1.0
		{
			return Ok(xz);
		}
		x += 4.0;
	}
	Err(anyhow::anyhow!("no outside-pad sample on the stored cell"))
}

impl Clone for OwnedDurham {
	fn clone(&self) -> Self {
		Self {
			snapshot: self.snapshot.clone(),
			layout: self.layout.clone(),
			fallback: self.fallback.clone(),
		}
	}
}

/// Ground whose `require_generation` is a no-op, so `finish` can name
/// [`VegetationGenerationCore`](vegetation_layer_model::VegetationGenerationCore)
/// without booting Durham.
struct Silent;

struct SilentCell;

impl terrain_layer_model::TerrainCell for SilentCell {
	type Mesh = durham::TerrainMeshBuilder;

	fn bounds(&self) -> Aabb3d {
		Aabb3d::new(Vec3::ZERO, Vec3::ZERO)
	}

	fn mesh_builder(&self) -> durham::TerrainMeshBuilder {
		use std::sync::Arc;

		use durham::{ComposedTerrain, TerrainSdf};
		use render_item::sdf::cpu_shot::{CpuShotBuilder, WallFaces};

		CpuShotBuilder::new(Arc::new(ComposedTerrain::from_terrain(TerrainSdf::new(1, 1.0))))
			.with_wall_faces(WallFaces::NONE)
	}

	fn chunk_pose(&self) -> bevy::prelude::Transform {
		bevy::prelude::Transform::IDENTITY
	}

	fn seeds_collision(&self) -> bool {
		false
	}

	fn res_2(&self) -> u8 {
		0
	}
}

#[derive(Clone)]
struct SilentSnapshot;

impl HeightField for SilentSnapshot {
	fn height_at(&self, _xz: Vec2) -> Option<f32> {
		None
	}

	fn fallback_height_at(&self, _xz: Vec2) -> f32 {
		0.0
	}
}

impl TerrainModel for Silent {
	type Base = Self;
	type Cell = SilentCell;
	type Read = ();
	type Snapshot = SilentSnapshot;
	type Prepare = ();

	fn prepare(
		_prepare: &mut bevy::ecs::system::SystemParamItem<'_, '_, Self::Prepare>,
		_bounds: Aabb3d,
		_lod_ref: &lod::lod_ref::LodRef,
	) {
	}

	fn height_at(
		_read: &bevy::ecs::system::SystemParamItem<'_, '_, Self::Read>,
		_xz: Vec2,
	) -> Option<f32> {
		None
	}

	fn fallback_height_at(
		_read: &bevy::ecs::system::SystemParamItem<'_, '_, Self::Read>,
		_xz: Vec2,
	) -> f32 {
		0.0
	}

	fn overlay_cell<'a>(
		_read: &'a bevy::ecs::system::SystemParamItem<'_, '_, Self::Read>,
		_bounds: Aabb3d,
		_target_size: f32,
		_overlay_size_tolerance: Option<f32>,
	) -> Option<&'a dyn terrain_layer_model::TerrainCell<Mesh = durham::TerrainMeshBuilder>>
	{
		None
	}

	fn snapshot(
		_read: &bevy::ecs::system::SystemParamItem<'_, '_, Self::Read>,
		_region: Aabb3d,
	) -> Self::Snapshot {
		SilentSnapshot
	}

	fn require_generation(_app: &App) {}
}

#[test]
#[should_panic(expected = "VegetationGenerationCore")]
fn presentation_without_generation_names_the_missing_plugin() {
	VegetationPresentationPlugin::<SilentMode, Silent>::default().finish(&mut App::new());
}

struct SilentMode;

impl GenerationMode for SilentMode {}

struct OtherMode;

impl GenerationMode for OtherMode {}

#[derive(SystemParam)]
struct ForestStateParam<'w, 's> {
	commands: bevy::prelude::Commands<'w, 's>,
	state: bevy::prelude::ResMut<'w, ForestPresenterState>,
}

impl RegionPresenter<ChicoGrove, ForestIndex> for ForestStateParam<'_, '_> {
	fn presented_version(&self, id: Id) -> Option<Version> {
		self.state.presented_version(id)
	}

	fn handle(&mut self, id: Id, _version: Version, _grove: &ChicoGrove, _lod_ref: &LodRef) {
		self.state.insert_presented(id, Vec::new());
	}

	fn hide(&mut self, id: Id) {
		self.state.hide(&mut self.commands, id);
	}

	fn is_hidden(&self, id: Id) -> bool {
		self.state.is_hidden(id)
	}

	fn presented_ids(&self) -> Vec<Id> {
		self.state.presented_ids()
	}

	fn remove_stale(&mut self, wanted: &HashSet<Id>) {
		self.state.remove_stale(&mut self.commands, wanted);
	}
}

fn forest_present_app(both: bool) -> (App, Id) {
	let mut app = App::new();
	app.add_plugins((
		MinimalPlugins,
		StatesPlugin,
		GenerationModePlugin::<SilentMode>::initial(),
		GenerationModePlugin::<OtherMode>::default(),
	));
	subscribe_mode::<(Silent, VegetationPresent), SilentMode>(&mut app);
	if both {
		subscribe_mode::<(Silent, VegetationPresent), OtherMode>(&mut app);
	}
	install_lod_present_gate::<(Silent, VegetationPresent), ForestLodChan>(&mut app);
	app.init_resource::<ForestPresenterState>();
	app.init_resource::<ForestIndex>();
	app.insert_resource({
		let mut keep = LodPresentKeepRegion::<ForestLodChan>::default();
		keep.region = Some(Aabb3d::from_min_max(Vec3::ZERO, Vec3::ONE));
		keep
	});
	app.add_plugins((
		LodPresentPlugin::<
			ChicoGrove,
			ForestIndex,
			ForestStateParam,
			ForestLodChan,
		>::default(),
		LodPresentCullPlugin::<
			ChicoGrove,
			ForestIndex,
			ForestStateParam,
			ForestLodChan,
		>::default(),
	));
	let id = Id::from_cell(Aabb3d::from_min_max(Vec3::ZERO, Vec3::ONE));
	let host = app.world_mut().spawn_empty().id();
	app.world_mut()
		.resource_mut::<ForestPresenterState>()
		.insert_presented(id, vec![host]);
	(app, id)
}

#[test]
fn losing_subscription_retires_and_returning_presents() -> anyhow::Result<()> {
	let (mut app, id) = forest_present_app(false);
	app.update();
	{
		let mut state =
			SystemState::<ModeSubscription<(Silent, VegetationPresent)>>::new(app.world_mut());
		anyhow::ensure!(
			state.get(app.world()).map_err(|error| anyhow::anyhow!("{error:?}"))?.active(),
			"subscribed mode presents"
		);
	}
	anyhow::ensure!(
		!app.world().resource::<ForestPresenterState>().presents(id),
		"an id missing from the index is stale"
	);

	app.world_mut()
		.resource_mut::<ForestPresenterState>()
		.insert_presented(id, Vec::new());
	app.world_mut()
		.resource_mut::<NextState<ActiveGenerationMode>>()
		.set(ActiveGenerationMode::of::<OtherMode>());
	app.update();
	{
		let mut state =
			SystemState::<ModeSubscription<(Silent, VegetationPresent)>>::new(app.world_mut());
		anyhow::ensure!(
			!state.get(app.world()).map_err(|error| anyhow::anyhow!("{error:?}"))?.active(),
			"unsubscribed mode retires"
		);
	}
	anyhow::ensure!(
		!app.world().resource::<LodPresentGate<ForestLodChan>>().open,
		"subscription closes the present gate"
	);
	anyhow::ensure!(
		app.world().resource::<LodPresentKeepRegion<ForestLodChan>>().region.is_none(),
		"unsubscribed keep cannot feed Produce"
	);
	anyhow::ensure!(
		!app.world().resource::<ForestPresenterState>().presents(id),
		"closing the gate retires presented groves"
	);

	app.world_mut()
		.resource_mut::<NextState<ActiveGenerationMode>>()
		.set(ActiveGenerationMode::of::<SilentMode>());
	app.update();
	{
		let mut state =
			SystemState::<ModeSubscription<(Silent, VegetationPresent)>>::new(app.world_mut());
		anyhow::ensure!(
			state.get(app.world()).map_err(|error| anyhow::anyhow!("{error:?}"))?.active(),
			"return presents again"
		);
	}
	anyhow::ensure!(
		app.world().resource::<LodPresentGate<ForestLodChan>>().open,
		"return opens the present gate"
	);
	Ok(())
}

#[test]
fn hop_out_and_back_does_not_keep_the_other_mode_presenters() -> anyhow::Result<()> {
	let (mut app, id) = forest_present_app(true);
	app.update();
	anyhow::ensure!(
		!app.world().resource::<ForestPresenterState>().presents(id),
		"OnExit-empty index retires the leaving mode's presenter"
	);

	app.world_mut()
		.resource_mut::<NextState<ActiveGenerationMode>>()
		.set(ActiveGenerationMode::of::<OtherMode>());
	app.update();
	app.world_mut()
		.resource_mut::<NextState<ActiveGenerationMode>>()
		.set(ActiveGenerationMode::of::<SilentMode>());
	app.update();
	anyhow::ensure!(
		!app.world().resource::<ForestPresenterState>().presents(id),
		"hop back does not restore the other mode's presenter"
	);
	Ok(())
}

#[test]
fn two_modes_install_the_core_once() -> anyhow::Result<()> {
	let mut app = App::new();
	app.add_plugins((MinimalPlugins, bevy::prelude::AssetPlugin::default(), StatesPlugin));
	app.add_plugins((
		GenerationModePlugin::<SilentMode>::initial(),
		GenerationModePlugin::<OtherMode>::default(),
		VegetationPresentationPlugin::<SilentMode, Silent>::default(),
		VegetationPresentationPlugin::<OtherMode, Silent>::default(),
	));
	anyhow::ensure!(
		app.is_plugin_added::<crate::VegetationPresentationCore<Silent>>(),
		"core is installed"
	);
	let subscribers = app.world().resource::<ModeSubscribers<(Silent, VegetationPresent)>>();
	anyhow::ensure!(subscribers.contains::<SilentMode>());
	anyhow::ensure!(subscribers.contains::<OtherMode>());
	Ok(())
}

#[test]
fn bump_out_cells_match_terrain_rings() {
	use chico::{BUMP_OUT_CELL_XZ, MEDIUM_BUMP_OUT_CELL_XZ};
	assert!((BUMP_OUT_CELL_XZ - TERRAIN_CELL_SIZE).abs() < 1e-3);
	assert!((MEDIUM_BUMP_OUT_CELL_XZ - 2.0 * TERRAIN_CELL_SIZE).abs() < 1e-3);
}

#[test]
fn chunk_ref_matches_cascade_chunk_for_cell() -> anyhow::Result<()> {
	use durham::cascade_chunk_for_cell;
	use lod_cascade::Chunk;
	use terrain_chunk_ref::TerrainChunkRef;
	use terrain_layer_model::TerrainCell;

	let mut world = World::new();
	let layout = TerrainCellLayout::default();
	let base = BaseTerrainNoise::from_config(&TerrainConfig::new(7));
	world.insert_resource(TerrainEntryStore::default());
	world
		.resource_mut::<TerrainEntryStore>()
		.insert_base_terrain_for_test(&layout, 0, 0, base);
	let store = world.resource::<TerrainEntryStore>();
	let probe = Aabb3d::from_min_max(Vec3::new(1.0, -10.0, 1.0), Vec3::new(2.0, 10.0, 2.0));
	let id = store
		.terrain_ids_overlapping(probe)
		.into_iter()
		.next()
		.ok_or_else(|| anyhow::anyhow!("stored cell"))?;
	let terrain = store.terrain(id).ok_or_else(|| anyhow::anyhow!("terrain"))?;
	let built = crate::present::chunk_ref(terrain);
	let cascade = cascade_chunk_for_cell(terrain.bounds(), terrain.res_2());
	let extent = match cascade.extent {
		Some(extent) => extent,
		None => Vec3::splat(cascade.size),
	};
	let chunk = Chunk::from_min_max(cascade.origin, cascade.origin + extent, None);
	let manual = TerrainChunkRef::new(terrain.mesh_builder(), chunk, terrain.res_2());
	assert_eq!(built.key(), manual.key());
	Ok(())
}
