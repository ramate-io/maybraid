use bevy::app::{App, Plugin};
use bevy::ecs::system::SystemState;
use bevy::math::bounding::Aabb3d;
use bevy::math::{IVec2, Vec2, Vec3};
use bevy::prelude::World;
use durham_terrain_models::{
	BaseTerrainNoise, Durham, DurhamTerrainConfig, TerrainCellLayout, TerrainConfig,
	TerrainEntryStore, WorldBaseTerrain,
};
use richmond_development_models::DevelopmentEntryStore;
use richmond_urbanization::UrbanizationIndex;
use terrain_layer_model::{
	BaseTerrainGenerationPlugin, BaseTerrainScheme, GenerationMode, GenerationModePlugin,
	HeightField, OnTerrain, TerrainModel, TerrainView,
};

struct TestMode;

impl GenerationMode for TestMode {}

impl BaseTerrainScheme<Durham> for TestMode {
	fn install(_app: &mut App, _config: &DurhamTerrainConfig) {}
}

impl crate::UrbanizationScheme<OnTerrain<Durham>> for TestMode {
	fn install(_app: &mut App, _config: &crate::UrbanizationLayerConfig) {}
}

use crate::Urbanization;

type UrbanizedDurham = Urbanization<OnTerrain<Durham>>;

fn empty_urbanized_world() -> World {
	let mut world = World::new();
	world.insert_resource(TerrainEntryStore::default());
	world.insert_resource(TerrainCellLayout::default());
	world.insert_resource(WorldBaseTerrain(BaseTerrainNoise::from_config(&TerrainConfig::new(42))));
	world.insert_resource(DevelopmentEntryStore::default());
	world.insert_resource(UrbanizationIndex::default());
	world
}

#[test]
fn without_pads_urbanization_reads_the_inner_surface() -> anyhow::Result<()> {
	let mut world = empty_urbanized_world();
	let base = BaseTerrainNoise::from_config(&TerrainConfig::new(42)).height_at(40.0, 25.0);
	let mut state = SystemState::<TerrainView<UrbanizedDurham>>::new(&mut world);
	let view = state.get(&world)?;

	assert_eq!(view.height_at(Vec2::new(40.0, 25.0)), None);
	assert_eq!(view.height_or_fallback(Vec2::new(40.0, 25.0)), base);
	assert!(view
		.cell_ids_overlapping(Aabb3d::new(Vec3::ZERO, Vec3::splat(1_000.0)))
		.is_empty());
	Ok(())
}

#[test]
#[should_panic(expected = "requires terrain_layer_model::generation::BaseTerrainGenerationCore")]
fn requirements_recurse_to_the_base_terrain() {
	UrbanizedDurham::require_generation(&App::new());
}

#[test]
fn default_urbanization_noise_parses() -> anyhow::Result<()> {
	use crate::DEFAULT_URBANIZATION_NOISE;
	use procedural_common::noise_params_from_scalar_str;

	let noise = noise_params_from_scalar_str(DEFAULT_URBANIZATION_NOISE)
		.map_err(|e| anyhow::anyhow!("{e}"))?;
	assert_eq!(noise.seed, 1337);
	assert!((noise.frequency - 0.0005).abs() < 1e-8);
	Ok(())
}

#[test]
fn parse_urbanization_kind_accepts_kebab() -> anyhow::Result<()> {
	use crate::parse_urbanization_kind;
	use richmond_urbanization::UrbanizationKind;

	assert_eq!(
		parse_urbanization_kind("frontier").map_err(|e| anyhow::anyhow!("{e}"))?,
		UrbanizationKind::Frontier
	);
	assert!(parse_urbanization_kind("not-a-city").is_err());
	Ok(())
}

#[test]
fn default_stream_radii_are_one_and_three_kilometres() -> anyhow::Result<()> {
	use crate::{stream_radii_m, DEFAULT_URBANIZATION_STREAM_RADIUS};
	use richmond_urbanization::{DEVELOPMENT_GENERATE_RADIUS_M, DEVELOPMENT_PRESENT_RADIUS_M};

	let (present, generate) = stream_radii_m(DEFAULT_URBANIZATION_STREAM_RADIUS);
	assert!((present - DEVELOPMENT_PRESENT_RADIUS_M).abs() < 1e-3);
	assert!((generate - DEVELOPMENT_GENERATE_RADIUS_M).abs() < 1e-3);
	Ok(())
}

#[test]
fn default_spec_matches_noise_string() -> anyhow::Result<()> {
	use crate::{
		UrbanizationStreamSpec, DEFAULT_URBANIZATION_NOISE, DEFAULT_URBANIZATION_STREAM_RADIUS,
	};
	use procedural_common::noise_params_from_scalar_str;

	let parsed = noise_params_from_scalar_str(DEFAULT_URBANIZATION_NOISE)
		.map_err(|e| anyhow::anyhow!("{e}"))?;
	let spec = UrbanizationStreamSpec::default();
	assert_eq!(spec.noise.seed, parsed.seed);
	assert!((spec.noise.frequency - parsed.frequency).abs() < 1e-8);
	assert_eq!(spec.stream_radius, DEFAULT_URBANIZATION_STREAM_RADIUS);
	Ok(())
}

#[test]
fn world_defaults_enable_urbanization_stream_at_budget_16() {
	use crate::{UrbanizationLayerConfig, DEFAULT_URBANIZATION_STREAM_RADIUS};

	let config = UrbanizationLayerConfig::world_defaults();
	assert!(config.urbanization.is_some());
	assert_eq!(
		config.urbanization.map(|s| s.stream_radius),
		Some(DEFAULT_URBANIZATION_STREAM_RADIUS)
	);
	assert_eq!(config.generate_budget, 16);
	assert_eq!(UrbanizationLayerConfig::default().generate_budget, 8);
}

#[test]
fn development_focus_from_kebab_is_case_insensitive() -> anyhow::Result<()> {
	use crate::DevelopmentFocus;

	anyhow::ensure!(DevelopmentFocus::from_kebab("Old-City-Market") == Some(DevelopmentFocus::OldCityMarket));
	anyhow::ensure!(DevelopmentFocus::from_kebab("  LES-HALLES  ") == Some(DevelopmentFocus::LesHalles));
	Ok(())
}

#[test]
fn stream_applies_focus_when_the_spec_kind_is_open() -> anyhow::Result<()> {
	use bevy::camera::Camera3d;
	use bevy::ecs::system::RunSystemOnce;
	use bevy::prelude::{MinimalPlugins, Transform};
	use bevy::state::app::StatesPlugin;
	use richmond_urbanization::{UrbanizationIndex, UrbanizationKind};
	use crate::{
		stream_urbanization, UrbanizationLayerConfig, UrbanizationModeConfig, UrbanizationStreamKey,
	};
	use crate::stream::register_urbanization_lod_generate;
	use terrain_layer_model::GenerationModePlugin;

	let mut config = UrbanizationLayerConfig::world_defaults();
	config.focus_urbanization = Some(UrbanizationKind::Frontier);
	anyhow::ensure!(
		config.urbanization.as_ref().is_some_and(|spec| spec.kind.is_none()),
		"world defaults leave kind open"
	);

	let mut app = App::new();
	app.add_plugins((MinimalPlugins, StatesPlugin));
	app.add_plugins(GenerationModePlugin::<TestMode>::initial());
	register_urbanization_lod_generate(&mut app, 16);
	app.insert_resource(UrbanizationModeConfig::<TestMode>::new(config));
	app.init_resource::<UrbanizationStreamKey>();
	app.world_mut()
		.spawn((Camera3d::default(), Transform::from_xyz(0.0, 8.0, 0.0)));

	app.world_mut()
		.run_system_once(stream_urbanization::<TestMode>)
		.map_err(|error| anyhow::anyhow!("{error:?}"))?;
	anyhow::ensure!(
		app.world().resource::<UrbanizationIndex>().kind == Some(UrbanizationKind::Frontier),
		"apply_spec must keep the focused kind"
	);
	Ok(())
}

#[test]
#[should_panic(expected = "requires terrain_layer_model::generation::BaseTerrainGenerationCore")]
fn urbanization_generation_without_base_names_the_missing_plugin() {
	use crate::UrbanizationGenerationPlugin;
	use durham_terrain_models::Durham;
	use terrain_layer_model::OnTerrain;

	// `build` installs Richmond plugins that need a full Bevy app. `finish`
	// is the requirement check the assemblers actually run.
	UrbanizationGenerationPlugin::<TestMode, OnTerrain<Durham>>::default().finish(&mut App::new());
}

#[test]
fn urbanization_stack_names_the_local_mode() {
	let _stack = (
		GenerationModePlugin::<TestMode>::initial(),
		BaseTerrainGenerationPlugin::<TestMode, Durham>::new(DurhamTerrainConfig::fine_patch(2)),
	);
}

struct FlatHeight(f32);

impl HeightField for FlatHeight {
	fn height_at(&self, _xz: Vec2) -> Option<f32> {
		Some(self.0)
	}

	fn fallback_height_at(&self, _xz: Vec2) -> f32 {
		self.0
	}
}

#[test]
fn pad_modulation_sets_exact_terrace_and_preserves_base_outside() -> anyhow::Result<()> {
	use crate::UrbanSnapshot;
	use richmond_development_models::{PadComplex, PadParams};

	let pad = PadComplex::building_skirt(Vec2::ZERO, Vec2::splat(10.0), 0.0, 12.0, PadParams::default());
	let snapshot = UrbanSnapshot::new(FlatHeight(3.0), pad);
	let terrace = snapshot.height_at(Vec2::ZERO).ok_or_else(|| anyhow::anyhow!("terrace"))?;
	let outside = snapshot
		.height_at(Vec2::new(1_000.0, 1_000.0))
		.ok_or_else(|| anyhow::anyhow!("outside"))?;
	assert!((terrace - 12.0).abs() < 1e-5);
	assert!((outside - 3.0).abs() < 1e-5);
	Ok(())
}

#[derive(bevy::prelude::Resource)]
struct OverlayPadSpec {
	source: lod::gen::Id,
	bounds: Aabb3d,
	res_2: u8,
}

fn insert_overlay_pad(
	mut index: richmond_development_models::DevelopmentIndex,
	spec: bevy::prelude::Res<OverlayPadSpec>,
) {
	use bevy::prelude::{Entity, Transform};
	use lod::gen::{Id, SpatialIndex};
	use lod::lod_ref::LodRef;
	use richmond_development_models::{PadComplex, TerrainWithPads};
	use terrain_layer_model::TerrainCell;

	let Some(terrain) = index.terrain.terrain(spec.source) else {
		return;
	};
	let mut padded = TerrainWithPads::compose(terrain, std::iter::empty::<&PadComplex>());
	padded.cell = spec.bounds;
	padded.res_2 = spec.res_2;
	let bounds = padded.bounds();
	let id = Id::from_cell(bounds);
	let transform = Transform::IDENTITY;
	let lod_ref = LodRef {
		entity: Entity::PLACEHOLDER,
		previous_transform: &transform,
		current_transform: &transform,
		bounds: &bounds,
	};
	SpatialIndex::<TerrainWithPads>::insert(&mut index, id, padded, bounds, &lod_ref);
}

fn overlay_width_res(
	world: &mut World,
	query: Aabb3d,
	target: f32,
	tolerance: Option<f32>,
) -> anyhow::Result<(f32, u8)> {
	let mut state = SystemState::<TerrainView<UrbanizedDurham>>::new(world);
	let view = state.get(world)?;
	let cell = view
		.overlay_cell(query, target, tolerance)
		.ok_or_else(|| anyhow::anyhow!("overlay cell"))?;
	let width = cell.bounds().max.x - cell.bounds().min.x;
	Ok((width, cell.res_2()))
}

#[test]
fn overlay_cell_prefers_a_padded_cell_then_falls_back_by_size() -> anyhow::Result<()> {
	use bevy::ecs::system::RunSystemOnce;
	use durham_terrain_models::TERRAIN_CELL_SIZE;
	use richmond_development_models::DevelopmentConfig;

	let mut world = empty_urbanized_world();
	world.insert_resource(DevelopmentConfig::default());
	let base = BaseTerrainNoise::from_config(&TerrainConfig::new(42));
	let fine = TerrainCellLayout::default();
	let medium = TerrainCellLayout {
		cell_size: 2.0 * TERRAIN_CELL_SIZE,
		..TerrainCellLayout::default()
	};
	{
		let mut store = world.resource_mut::<TerrainEntryStore>();
		store.insert_base_terrain_for_test(&fine, 0, 0, base.clone());
		store.insert_base_terrain_for_test(&medium, 0, 0, base);
	}
	let query = Aabb3d::from_min_max(Vec3::new(1.0, -10.0, 1.0), Vec3::new(20.0, 10.0, 20.0));
	let (source, fine_bounds, medium_bounds) = {
		let store = world.resource::<TerrainEntryStore>();
		let mut source = None;
		let mut fine_bounds = None;
		let mut medium_bounds = None;
		for id in store.terrain_ids_overlapping(query) {
			let Some(terrain) = store.terrain(id) else {
				continue;
			};
			let width = terrain.cell.max.x - terrain.cell.min.x;
			if (width - TERRAIN_CELL_SIZE).abs() < 1.0 {
				source = Some(id);
				fine_bounds = Some(terrain.cell);
			}
			if (width - 2.0 * TERRAIN_CELL_SIZE).abs() < 1.0 {
				medium_bounds = Some(terrain.cell);
			}
		}
		(
			source.ok_or_else(|| anyhow::anyhow!("fine source"))?,
			fine_bounds.ok_or_else(|| anyhow::anyhow!("fine bounds"))?,
			medium_bounds.ok_or_else(|| anyhow::anyhow!("medium bounds"))?,
		)
	};

	let run = |world: &mut World| {
		world
			.run_system_once(insert_overlay_pad)
			.map_err(|error| anyhow::anyhow!("{error:?}"))
	};

	// Urbanized fine: any padded size wins.
	world.insert_resource(OverlayPadSpec { source, bounds: medium_bounds, res_2: 9 });
	run(&mut world)?;
	let (width, res) = overlay_width_res(&mut world, query, TERRAIN_CELL_SIZE, None)?;
	assert!((width - 2.0 * TERRAIN_CELL_SIZE).abs() < 1e-3);
	assert_eq!(res, 9);

	// Padded absent: best-sized raw cell.
	world.resource_mut::<DevelopmentEntryStore>().clear();
	let (width, res) = overlay_width_res(&mut world, query, TERRAIN_CELL_SIZE, None)?;
	assert!((width - TERRAIN_CELL_SIZE).abs() < 1e-3);
	assert_eq!(res, 0);

	// Urbanized medium rejects a padded cell of the wrong size.
	world.insert_resource(OverlayPadSpec { source, bounds: fine_bounds, res_2: 9 });
	run(&mut world)?;
	let (width, res) = overlay_width_res(&mut world, query, 2.0 * TERRAIN_CELL_SIZE, Some(1e-2))?;
	assert!((width - 2.0 * TERRAIN_CELL_SIZE).abs() < 1e-3);
	assert_eq!(res, 0);

	// Urbanized medium accepts a padded cell within 1e-2 of the medium width.
	world.resource_mut::<DevelopmentEntryStore>().clear();
	world.insert_resource(OverlayPadSpec { source, bounds: medium_bounds, res_2: 9 });
	run(&mut world)?;
	let (width, res) = overlay_width_res(&mut world, query, 2.0 * TERRAIN_CELL_SIZE, Some(1e-2))?;
	assert!((width - 2.0 * TERRAIN_CELL_SIZE).abs() < 1e-3);
	assert_eq!(res, 9);
	Ok(())
}

#[test]
fn empty_keep_draws_a_fine_patch_from_the_layout() -> anyhow::Result<()> {
	use durham_terrain_models::fine_patch_cell_layout;
	use crate::{urbanization_host_region, urbanization_visual_region};

	let layout = fine_patch_cell_layout(2, IVec2::ZERO);
	let visual = urbanization_visual_region(&layout, None)
		.ok_or_else(|| anyhow::anyhow!("visual region"))?;
	let host = urbanization_host_region(&layout, None)
		.ok_or_else(|| anyhow::anyhow!("host region"))?;
	anyhow::ensure!(visual == layout.presentation_region());
	anyhow::ensure!(host == layout.presentation_region());
	Ok(())
}

#[test]
fn host_region_covers_every_leaf_of_selected_cells() -> anyhow::Result<()> {
	use std::collections::HashSet;

	use bevy::ecs::system::RunSystemOnce;
	use lod::gen::{Id, SpatialIndex};
	use lod::presentation::LodPresentKeepRegion;
	use procedural_common::NoiseParams;
	use bevy::math::bounding::IntersectsVolume;
	use richmond_urbanization::{
		SelectedUrbanization, UrbanDevelopmentKind, UrbanizationExtent, UrbanizationIndex,
		UrbanizationKind, UrbanizationLodChan,
	};

	use crate::{write_urbanization_host_region, UrbanizationLayerRegion};

	let extent = UrbanizationExtent::default_cell();
	let keep = Aabb3d::from_min_max(
		Vec3::new(-10.0, 0.0, -10.0),
		Vec3::new(10.0, 1.0, 10.0),
	);
	anyhow::ensure!(
		keep.intersects(&extent.aabb()),
		"the keep must overlap the urbanization cell"
	);

	let mut world = World::new();
	world.insert_resource(UrbanizationLayerRegion::default());
	world.insert_resource({
		let mut keep_region = LodPresentKeepRegion::<UrbanizationLodChan>::default();
		keep_region.region = Some(keep);
		keep_region
	});
	world.insert_resource(UrbanizationIndex::default());
	world.resource_mut::<UrbanizationIndex>().kind = Some(UrbanizationKind::MixedAgeCity);
	world.resource_mut::<UrbanizationIndex>().ensure_selected(extent, NoiseParams::default());

	let selected = world
		.resource::<UrbanizationIndex>()
		.get(extent.id())
		.cloned()
		.ok_or_else(|| anyhow::anyhow!("selected cell"))?;
	let old: HashSet<Id> = selected
		.leaves
		.iter()
		.filter(|leaf| leaf.kind != UrbanDevelopmentKind::Empty)
		.map(richmond_urbanization::DevelopmentLeaf::id)
		.collect();
	anyhow::ensure!(!old.is_empty(), "hopscotch produced no filled leaves");
	let outside_keep = selected.leaves.iter().any(|leaf| {
		leaf.kind != UrbanDevelopmentKind::Empty && !keep.intersects(&leaf.bounds)
	});
	anyhow::ensure!(
		outside_keep,
		"this fixture needs a filled leaf that sits outside the keep"
	);

	world
		.run_system_once(write_urbanization_host_region)
		.map_err(|error| anyhow::anyhow!("{error:?}"))?;
	let host = world
		.resource::<UrbanizationLayerRegion>()
		.region
		.ok_or_else(|| anyhow::anyhow!("host region"))?;
	anyhow::ensure!(host == extent.aabb(), "the stream writes the selected cell");

	let index = world.resource::<UrbanizationIndex>();
	let new: HashSet<Id> = SpatialIndex::<SelectedUrbanization>::tracked_ids_for(&*index, host)
		.into_iter()
		.filter_map(|tracked| index.get(tracked.0))
		.flat_map(|selected| selected.leaves.iter())
		.filter(|leaf| leaf.kind != UrbanDevelopmentKind::Empty)
		.map(richmond_urbanization::DevelopmentLeaf::id)
		.collect();
	anyhow::ensure!(old == new, "hosts present the same leaf ids as the old walk");
	Ok(())
}

struct StreamMode;
struct OtherMode;

impl GenerationMode for StreamMode {}
impl GenerationMode for OtherMode {}

impl BaseTerrainScheme<Durham> for StreamMode {
	fn install(_app: &mut App, _config: &DurhamTerrainConfig) {}
}

impl BaseTerrainScheme<Durham> for OtherMode {
	fn install(_app: &mut App, _config: &DurhamTerrainConfig) {}
}

impl crate::UrbanizationScheme<OnTerrain<Durham>> for StreamMode {
	fn install(_app: &mut App, _config: &crate::UrbanizationLayerConfig) {}
}

impl crate::UrbanizationScheme<OnTerrain<Durham>> for OtherMode {
	fn install(_app: &mut App, _config: &crate::UrbanizationLayerConfig) {}
}

#[test]
fn leaving_a_stream_mode_clears_then_reentering_streams_again() -> anyhow::Result<()> {
	use bevy::camera::Camera3d;
	use bevy::ecs::message::Messages;
	use bevy::ecs::system::RunSystemOnce;
	use bevy::prelude::{MinimalPlugins, NextState, OnExit, Transform};
	use bevy::state::app::StatesPlugin;
	use lod::gen::{Id, LodGenerateRegion};
	use richmond_development_models::{DevelopmentCell, DevelopmentConfig, DevelopmentEntryStore};
	use richmond_urbanization::{UrbanizationExtent, UrbanizationIndex, UrbanizationLodChan};
	use crate::generation::clear_urbanization_mode;
	use crate::stream::register_urbanization_lod_generate;
	use crate::{
		stream_urbanization, UrbanizationLayerConfig, UrbanizationLayerRegion, UrbanizationModeConfig,
		UrbanizationStreamKey, UrbanizationStreamSpec,
	};
	use terrain_layer_model::{ActiveGenerationMode, GenerationModePlugin};

	let spec = UrbanizationStreamSpec::default();
	let mut app = App::new();
	app.add_plugins((MinimalPlugins, StatesPlugin));
	app.add_plugins((
		GenerationModePlugin::<StreamMode>::initial(),
		GenerationModePlugin::<OtherMode>::default(),
	));
	register_urbanization_lod_generate(&mut app, 16);
	app.insert_resource(UrbanizationModeConfig::<StreamMode>::new(
		UrbanizationLayerConfig::world_defaults(),
	));
	app.init_resource::<UrbanizationStreamKey>();
	app.init_resource::<UrbanizationLayerRegion>();
	app.init_resource::<DevelopmentEntryStore>();
	app.init_resource::<DevelopmentConfig>();
	app.add_systems(
		OnExit(ActiveGenerationMode::of::<StreamMode>()),
		clear_urbanization_mode,
	);
	app.add_systems(
		OnExit(ActiveGenerationMode::of::<OtherMode>()),
		clear_urbanization_mode,
	);
	app.world_mut()
		.spawn((Camera3d::default(), Transform::from_xyz(0.0, 8.0, 0.0)));

	let bounds = Aabb3d::from_min_max(
		Vec3::new(4_000.0, 0.0, 4_000.0),
		Vec3::new(4_080.0, 1.0, 4_080.0),
	);
	let streamed_id = Id::from_cell(bounds);
	app.world_mut()
		.resource_mut::<DevelopmentEntryStore>()
		.insert_cell(streamed_id, DevelopmentCell::empty(bounds));
	let extent = UrbanizationExtent::default_cell();
	app.world_mut()
		.resource_mut::<UrbanizationIndex>()
		.ensure_selected(extent, spec.noise);
	let selected = extent.id();
	app.world_mut().insert_resource(UrbanizationStreamKey(Some(spec.key())));

	app.update();
	anyhow::ensure!(app.world().resource::<DevelopmentEntryStore>().cell(streamed_id).is_some());

	app.world_mut()
		.resource_mut::<NextState<ActiveGenerationMode>>()
		.set(ActiveGenerationMode::of::<OtherMode>());
	app.update();
	anyhow::ensure!(
		app.world().resource::<DevelopmentEntryStore>().cell(streamed_id).is_none(),
		"the other mode holds no streamed development"
	);
	anyhow::ensure!(
		app.world().resource::<UrbanizationIndex>().get(selected).is_none(),
		"the other mode holds no hopscotch selection"
	);
	anyhow::ensure!(app.world().resource::<UrbanizationStreamKey>().0.is_none());

	app.world_mut()
		.resource_mut::<NextState<ActiveGenerationMode>>()
		.set(ActiveGenerationMode::of::<StreamMode>());
	app.update();
	app.world_mut()
		.run_system_once(stream_urbanization::<StreamMode>)
		.map_err(|error| anyhow::anyhow!("{error:?}"))?;
	anyhow::ensure!(
		app.world().resource::<UrbanizationStreamKey>().0.as_ref() == Some(&spec.key()),
		"re-entering stores the spec key again"
	);
	let regions = app
		.world()
		.resource::<Messages<LodGenerateRegion<UrbanizationLodChan>>>()
		.iter_current_update_messages()
		.count();
	anyhow::ensure!(regions == 1, "re-entering emits a generate region, got {regions}");
	Ok(())
}
