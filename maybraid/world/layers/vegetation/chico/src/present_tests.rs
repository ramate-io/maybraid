use std::collections::HashSet;

use bevy::ecs::system::{SystemParam, SystemState};
use bevy::math::bounding::Aabb3d;
use bevy::math::{Vec2, Vec3};
use bevy::prelude::{App, MinimalPlugins, NextState, World};
use bevy::state::app::StatesPlugin;
use durham::{
	BaseTerrainNoise, Durham, HcsgStorage, TerrainCellLayout, TerrainConfig, TerrainHeightSnapshot,
	TerrainStorage, WorldBaseTerrain,
};
use layer_stack::{
	install_lod_present_gate, subscribe_mode, ActiveGenerationMode, GenerationMode,
	GenerationModePlugin, ModeSubscription,
};
use lod::gen::{Id, Version};
use lod::lod_ref::LodRef;
use lod::presentation::RegionPresenter;
use lod::{LodPresentCullPlugin, LodPresentGate, LodPresentKeepRegion, LodPresentPlugin};
use lod::hcsg::universal_bounds;
use richmond::{
	register_richmond_nodes, AuthoredDevelopment, AuthoredDevelopments, DevelopmentConfig,
	DevelopmentKind, PadComplex, RichmondDevelopment, RichmondStorage,
};
use terrain_layer_model::{HeightField, OnTerrain, TerrainView};
use urbanization_layer_model::Urbanization;
use vegetation_groves::{
	GroveHeightModulation, GroveTerrain, GroveWorldSample, ModulatedGroveSample,
};
use vegetation_layer_model::Vegetation;
use vegetation_layer_presentation::VegetationPresent;

use crate::generation::ForestLodChan;
use crate::grove::ChicoGrove;
use crate::index::ForestIndex;
use crate::layer_present::GroundGroveSample;
use crate::model::Chico;
use crate::present::ForestPresenterState;

type Urbanized = Urbanization<richmond::Richmond<OnTerrain<Durham>>>;
type Stacked = Vegetation<Chico<Urbanized>>;

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
	anyhow::ensure!(old_height == new_height, "height at {xz:?}: {old_height} vs {new_height}");
	let old_steep = old.steepness_at(position);
	let new_steep = new.steepness_at(position);
	anyhow::ensure!(old_steep == new_steep, "steepness at {xz:?}: {old_steep} vs {new_steep}");
	Ok(())
}

#[test]
fn ground_grove_sample_matches_durham_and_modulated_samples() -> anyhow::Result<()> {
	let mut world = World::new();
	let layout = TerrainCellLayout::default();
	let base = BaseTerrainNoise::from_config(&TerrainConfig::new(42));
	let mut storage = HcsgStorage::default();
	register_richmond_nodes::<OnTerrain<Durham>>(&mut storage);
	storage.insert_base_terrain_for_test(&layout, 0, 0, base.clone());
	world.insert_resource(storage);
	world.insert_resource(layout.clone());
	world.insert_resource(WorldBaseTerrain(base.clone()));

	let store = world.resource::<HcsgStorage>();
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
	let authored = AuthoredDevelopment {
		cell: region,
		kinds: vec![DevelopmentKind::LesHalles],
		height: 12.0,
		config: config.clone(),
		courtyard: None,
	};
	{
		let mut storage = world.resource_mut::<HcsgStorage>();
		storage.seed(config, universal_bounds());
		storage.seed(AuthoredDevelopments(vec![authored]), universal_bounds());
		storage
			.get_or_generate::<RichmondDevelopment<OnTerrain<Durham>>>(Id::from_cell(region))
			.ok_or_else(|| anyhow::anyhow!("authored development"))?;
	}

	let owned = OwnedDurham {
		snapshot: world.resource::<HcsgStorage>().height_snapshot(),
		layout: layout.clone(),
		fallback: base,
	};
	let pads = world.resource::<HcsgStorage>().merged_pads::<OnTerrain<Durham>>(region);
	let old_durham = DurhamGroveSample(owned.clone());
	let old_urban =
		ModulatedGroveSample::new(DurhamGroveSample(owned), vec![DevelopmentPadModulation(pads)]);

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

	let center =
		Vec2::new((region.min.x + region.max.x) * 0.5, (region.min.z + region.max.z) * 0.5);
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
	subscribe_mode::<(Stacked, VegetationPresent), SilentMode>(&mut app);
	if both {
		subscribe_mode::<(Stacked, VegetationPresent), OtherMode>(&mut app);
	}
	install_lod_present_gate::<(Stacked, VegetationPresent), ForestLodChan>(&mut app);
	app.init_resource::<ForestPresenterState>();
	app.init_resource::<ForestIndex>();
	app.insert_resource({
		let mut keep = LodPresentKeepRegion::<ForestLodChan>::default();
		keep.region = Some(Aabb3d::from_min_max(Vec3::ZERO, Vec3::ONE));
		keep
	});
	app.add_plugins((
		LodPresentPlugin::<ChicoGrove, ForestIndex, ForestStateParam, ForestLodChan>::default(),
		LodPresentCullPlugin::<ChicoGrove, ForestIndex, ForestStateParam, ForestLodChan>::default(),
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
			SystemState::<ModeSubscription<(Stacked, VegetationPresent)>>::new(app.world_mut());
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
			SystemState::<ModeSubscription<(Stacked, VegetationPresent)>>::new(app.world_mut());
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
			SystemState::<ModeSubscription<(Stacked, VegetationPresent)>>::new(app.world_mut());
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
fn closing_the_present_gate_despawns_partially_spawned_hosts() -> anyhow::Result<()> {
	let (mut app, id) = forest_present_app(false);
	app.update();
	let host = app.world_mut().spawn_empty().id();
	app.world_mut()
		.resource_mut::<ForestPresenterState>()
		.insert_growing_hosts(id, vec![host]);
	app.world_mut()
		.resource_mut::<NextState<ActiveGenerationMode>>()
		.set(ActiveGenerationMode::of::<OtherMode>());
	app.update();
	anyhow::ensure!(
		!app.world().resource::<LodPresentGate<ForestLodChan>>().open,
		"leaving the subscribed mode closes the present gate"
	);
	anyhow::ensure!(
		app.world().get_entity(host).is_err(),
		"gate close must despawn in-flight hosts, not leave them queued for cull"
	);
	app.update();
	anyhow::ensure!(
		!app.world().resource::<LodPresentGate<ForestLodChan>>().open,
		"the gate stays closed until a subscribed mode returns"
	);
	anyhow::ensure!(
		app.world().get_entity(host).is_err(),
		"hosts stay gone while the gate remains closed"
	);
	Ok(())
}
