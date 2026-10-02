use std::collections::HashSet;

use bevy::app::App;
use bevy::ecs::system::{RunSystemOnce, SystemState};
use bevy::math::bounding::{Aabb3d, IntersectsVolume};
use bevy::math::{Vec2, Vec3};
use bevy::prelude::{
	AssetPlugin, Camera3d, Entity, MinimalPlugins, NextState, OnExit, Transform, Visibility, World,
};
use bevy::state::app::StatesPlugin;
use durham::{
	fine_patch_cell_layout, playable_world_cell_layout, BaseTerrainNoise, Durham,
	DurhamTerrainConfig, PresentedTerrainScene, TerrainCellLayout, TerrainColliderMeshSource,
	TerrainConfig, TerrainEntryStore, TerrainSuperseded, TerrainTrimeshCollider, WorldBaseTerrain,
	TERRAIN_CELL_SIZE,
};
use layer_stack::{ActiveGenerationMode, GenerationMode, GenerationModePlugin};
use lod::gen::{Id, LodGenerateBudget, LodGenerateRegion, SpatialIndex};
use lod::lod_ref::LodRef;
use lod::presentation::LodPresentKeepRegion;
use procedural_common::{noise_params_from_scalar_str, NoiseParams};
use terrain_layer_model::{
	BaseTerrainGenerationCore, BaseTerrainGenerationPlugin, BaseTerrainScheme, HeightField,
	OnTerrain, TerrainExtent, TerrainStreaming, TerrainView,
};
use urbanization_cells::{
	DevelopmentLeaf, SelectedUrbanization, UrbanDevelopmentKind, UrbanizationExtent,
	UrbanizationIndex, UrbanizationKind, UrbanizationLodChan,
};
use urbanization_layer_model::{
	urbanization_host_region, UrbanModel, UrbanSnapshot, Urbanization, UrbanizationGeneration,
	UrbanizationGenerationCore, UrbanizationGenerationPlugin, UrbanizationLayerRegion,
	UrbanizationModeConfig, UrbanizationScheme,
};
use urbanization_layer_presentation::{
	PaddedCells, UrbanizationHosts, UrbanizationPresentationPlugin,
};

use crate::index::DevelopmentIndex;
use crate::layer::Richmond;
use crate::layer_config::{
	DevelopmentFocus, RichmondConfig, UrbanizationStreamSpec, DEFAULT_URBANIZATION_NOISE,
	DEFAULT_URBANIZATION_STREAM_RADIUS, PLAYGROUND_LIKELIHOOD,
};
use crate::layer_present::{
	present_richmond_hosts, quantize_viewer_xz_for_test, sync_raw_terrain_replacements,
	UrbanizationPaddedTerrainState, UrbanizationPresenterState,
};
use crate::layer_stream::{
	parse_urbanization_kind, register_urbanization_lod_generate, stream_radii_m,
	stream_urbanization, write_urbanization_host_region, UrbanizationStreamKey,
};
use crate::padded::{PresentedPaddedTerrainScene, TerrainWithPads};
use crate::{DevelopmentCell, DevelopmentConfig, DevelopmentEntryStore, PadComplex, PadParams};

struct TestMode;

impl GenerationMode for TestMode {}

impl BaseTerrainScheme<Durham> for TestMode {
	fn install(_app: &mut App, _config: &DurhamTerrainConfig) {}
}

impl UrbanizationScheme<Richmond<OnTerrain<Durham>>> for TestMode {
	fn install(_app: &mut App, _config: &RichmondConfig) {}
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

impl UrbanizationScheme<Richmond<OnTerrain<Durham>>> for StreamMode {
	fn install(_app: &mut App, _config: &RichmondConfig) {}
}

impl UrbanizationScheme<Richmond<OnTerrain<Durham>>> for OtherMode {
	fn install(_app: &mut App, _config: &RichmondConfig) {}
}

type Urbanized = Urbanization<Richmond<OnTerrain<Durham>>>;

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
	let mut state = SystemState::<TerrainView<Urbanized>>::new(&mut world);
	let view = state.get(&world).map_err(|error| anyhow::anyhow!("{error:?}"))?;

	anyhow::ensure!(view.height_at(Vec2::new(40.0, 25.0)).is_none());
	anyhow::ensure!(view.height_or_fallback(Vec2::new(40.0, 25.0)) == base);
	Ok(())
}

#[test]
fn default_urbanization_noise_parses() -> anyhow::Result<()> {
	let noise = noise_params_from_scalar_str(DEFAULT_URBANIZATION_NOISE)
		.map_err(|error| anyhow::anyhow!("{error}"))?;
	anyhow::ensure!(noise.seed == 1337);
	anyhow::ensure!((noise.frequency - 0.0005).abs() < 1e-8);
	Ok(())
}

#[test]
fn parse_urbanization_kind_accepts_kebab() -> anyhow::Result<()> {
	anyhow::ensure!(
		parse_urbanization_kind("frontier").map_err(|error| anyhow::anyhow!("{error}"))?
			== UrbanizationKind::Frontier
	);
	anyhow::ensure!(parse_urbanization_kind("not-a-city").is_err());
	Ok(())
}

#[test]
fn default_stream_radii_are_one_and_three_kilometres() -> anyhow::Result<()> {
	use urbanization_cells::{DEVELOPMENT_GENERATE_RADIUS_M, DEVELOPMENT_PRESENT_RADIUS_M};

	let (present, generate) = stream_radii_m(DEFAULT_URBANIZATION_STREAM_RADIUS);
	anyhow::ensure!((present - DEVELOPMENT_PRESENT_RADIUS_M).abs() < 1e-3);
	anyhow::ensure!((generate - DEVELOPMENT_GENERATE_RADIUS_M).abs() < 1e-3);
	Ok(())
}

#[test]
fn default_spec_matches_noise_string() -> anyhow::Result<()> {
	let parsed = noise_params_from_scalar_str(DEFAULT_URBANIZATION_NOISE)
		.map_err(|error| anyhow::anyhow!("{error}"))?;
	let spec = UrbanizationStreamSpec::default();
	anyhow::ensure!(spec.noise.seed == parsed.seed);
	anyhow::ensure!((spec.noise.frequency - parsed.frequency).abs() < 1e-8);
	anyhow::ensure!(spec.stream_radius == DEFAULT_URBANIZATION_STREAM_RADIUS);
	Ok(())
}

#[test]
fn world_defaults_enable_urbanization_stream_at_budget_16() -> anyhow::Result<()> {
	let config = RichmondConfig::world_defaults();
	anyhow::ensure!(config.urbanization.is_some());
	anyhow::ensure!(
		config.urbanization.map(|spec| spec.stream_radius)
			== Some(DEFAULT_URBANIZATION_STREAM_RADIUS)
	);
	anyhow::ensure!(config.generate_budget == 16);
	anyhow::ensure!(RichmondConfig::default().generate_budget == 8);
	Ok(())
}

#[test]
fn development_focus_from_kebab_is_case_insensitive() -> anyhow::Result<()> {
	anyhow::ensure!(
		DevelopmentFocus::from_kebab("Old-City-Market") == Some(DevelopmentFocus::OldCityMarket)
	);
	anyhow::ensure!(
		DevelopmentFocus::from_kebab("  LES-HALLES  ") == Some(DevelopmentFocus::LesHalles)
	);
	Ok(())
}

#[test]
fn stream_applies_focus_when_the_spec_kind_is_open() -> anyhow::Result<()> {
	let mut config = RichmondConfig::world_defaults();
	config.focus_urbanization = Some(UrbanizationKind::Frontier);
	anyhow::ensure!(
		config.urbanization.as_ref().is_some_and(|spec| spec.kind.is_none()),
		"world defaults leave kind open"
	);

	let mut app = App::new();
	app.add_plugins((MinimalPlugins, StatesPlugin));
	app.add_plugins(GenerationModePlugin::<TestMode>::initial());
	register_urbanization_lod_generate(&mut app);
	app.insert_resource(UrbanizationModeConfig::<TestMode, Richmond<OnTerrain<Durham>>>::new(
		config,
	));
	app.init_resource::<UrbanizationStreamKey>();
	app.world_mut().spawn((Camera3d::default(), Transform::from_xyz(0.0, 8.0, 0.0)));

	app.world_mut()
		.run_system_once(stream_urbanization::<TestMode, OnTerrain<Durham>>)
		.map_err(|error| anyhow::anyhow!("{error:?}"))?;
	anyhow::ensure!(
		app.world().resource::<UrbanizationIndex>().kind == Some(UrbanizationKind::Frontier),
		"apply_spec must keep the focused kind"
	);
	Ok(())
}

#[test]
fn urbanization_stack_names_the_local_mode() {
	let _stack = (
		GenerationModePlugin::<TestMode>::initial(),
		BaseTerrainGenerationPlugin::<TestMode, Durham>::new(DurhamTerrainConfig::fine_patch(2)),
		UrbanizationGenerationPlugin::<TestMode, Richmond<OnTerrain<Durham>>>::new(
			RichmondConfig::default(),
		),
		UrbanizationPresentationPlugin::<TestMode, Richmond<OnTerrain<Durham>>>::default(),
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
	let pad =
		PadComplex::building_skirt(Vec2::ZERO, Vec2::splat(10.0), 0.0, 12.0, PadParams::default());
	let snapshot = UrbanSnapshot::new(FlatHeight(3.0), pad);
	let terrace = snapshot.height_at(Vec2::ZERO).ok_or_else(|| anyhow::anyhow!("terrace"))?;
	let outside = snapshot
		.height_at(Vec2::new(1_000.0, 1_000.0))
		.ok_or_else(|| anyhow::anyhow!("outside"))?;
	anyhow::ensure!((terrace - 12.0).abs() < 1e-5);
	anyhow::ensure!((outside - 3.0).abs() < 1e-5);
	Ok(())
}

#[derive(bevy::prelude::Resource)]
struct OverlayPadSpec {
	source: Id,
	bounds: Aabb3d,
	res_2: u8,
}

fn insert_overlay_pad(
	mut index: DevelopmentIndex<OnTerrain<Durham>>,
	spec: bevy::prelude::Res<OverlayPadSpec>,
) {
	use crate::ground::RichmondGround;
	use terrain_layer_model::TerrainCell;

	let Some(terrain) = OnTerrain::<Durham>::stored_cell(&index.ground, spec.source) else {
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
	let mut state = SystemState::<TerrainView<Urbanized>>::new(world);
	let view = state.get(world).map_err(|error| anyhow::anyhow!("{error:?}"))?;
	let cell = view
		.overlay_cell(query, target, tolerance)
		.ok_or_else(|| anyhow::anyhow!("overlay cell"))?;
	let width = cell.bounds().max.x - cell.bounds().min.x;
	Ok((width, cell.res_2()))
}

#[test]
fn overlay_cell_prefers_a_padded_cell_then_falls_back_by_size() -> anyhow::Result<()> {
	let mut world = empty_urbanized_world();
	world.insert_resource(DevelopmentConfig::default());
	let base = BaseTerrainNoise::from_config(&TerrainConfig::new(42));
	let fine = TerrainCellLayout::default();
	let medium =
		TerrainCellLayout { cell_size: 2.0 * TERRAIN_CELL_SIZE, ..TerrainCellLayout::default() };
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

	world.insert_resource(OverlayPadSpec { source, bounds: medium_bounds, res_2: 9 });
	run(&mut world)?;
	let (width, res) = overlay_width_res(&mut world, query, TERRAIN_CELL_SIZE, None)?;
	anyhow::ensure!((width - 2.0 * TERRAIN_CELL_SIZE).abs() < 1e-3);
	anyhow::ensure!(res == 9);

	world.resource_mut::<DevelopmentEntryStore>().clear();
	let (width, res) = overlay_width_res(&mut world, query, TERRAIN_CELL_SIZE, None)?;
	anyhow::ensure!((width - TERRAIN_CELL_SIZE).abs() < 1e-3);
	anyhow::ensure!(res == 0);

	world.insert_resource(OverlayPadSpec { source, bounds: fine_bounds, res_2: 9 });
	run(&mut world)?;
	let (width, res) = overlay_width_res(&mut world, query, 2.0 * TERRAIN_CELL_SIZE, Some(1e-2))?;
	anyhow::ensure!((width - 2.0 * TERRAIN_CELL_SIZE).abs() < 1e-3);
	anyhow::ensure!(res == 0);

	world.resource_mut::<DevelopmentEntryStore>().clear();
	world.insert_resource(OverlayPadSpec { source, bounds: medium_bounds, res_2: 9 });
	run(&mut world)?;
	let (width, res) = overlay_width_res(&mut world, query, 2.0 * TERRAIN_CELL_SIZE, Some(1e-2))?;
	anyhow::ensure!((width - 2.0 * TERRAIN_CELL_SIZE).abs() < 1e-3);
	anyhow::ensure!(res == 9);
	Ok(())
}

#[test]
fn host_region_covers_every_leaf_of_selected_cells() -> anyhow::Result<()> {
	let extent = UrbanizationExtent::default_cell();
	let keep = Aabb3d::from_min_max(Vec3::new(-10.0, 0.0, -10.0), Vec3::new(10.0, 1.0, 10.0));
	anyhow::ensure!(keep.intersects(&extent.aabb()), "the keep must overlap the urbanization cell");

	let mut world = World::new();
	world.insert_resource(UrbanizationLayerRegion::default());
	world.insert_resource({
		let mut keep_region = LodPresentKeepRegion::<UrbanizationLodChan>::default();
		keep_region.region = Some(keep);
		keep_region
	});
	world.insert_resource(UrbanizationIndex::default());
	world.resource_mut::<UrbanizationIndex>().kind = Some(UrbanizationKind::MixedAgeCity);
	world
		.resource_mut::<UrbanizationIndex>()
		.ensure_selected(extent, NoiseParams::default());

	let selected = world
		.resource::<UrbanizationIndex>()
		.get(extent.id())
		.cloned()
		.ok_or_else(|| anyhow::anyhow!("selected cell"))?;
	let old: HashSet<Id> = selected
		.leaves
		.iter()
		.filter(|leaf| leaf.kind != UrbanDevelopmentKind::Empty)
		.map(DevelopmentLeaf::id)
		.collect();
	anyhow::ensure!(!old.is_empty(), "hopscotch produced no filled leaves");
	let outside_keep = selected
		.leaves
		.iter()
		.any(|leaf| leaf.kind != UrbanDevelopmentKind::Empty && !keep.intersects(&leaf.bounds));
	anyhow::ensure!(outside_keep, "this fixture needs a filled leaf that sits outside the keep");

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
		.map(DevelopmentLeaf::id)
		.collect();
	anyhow::ensure!(old == new, "hosts present the same leaf ids as the old walk");
	Ok(())
}

#[test]
fn leaving_a_stream_mode_clears_then_reentering_streams_again() -> anyhow::Result<()> {
	use bevy::ecs::message::Messages;

	let spec = UrbanizationStreamSpec::default();
	let mut app = App::new();
	app.add_plugins((MinimalPlugins, StatesPlugin));
	app.add_plugins((
		GenerationModePlugin::<StreamMode>::initial(),
		GenerationModePlugin::<OtherMode>::default(),
	));
	register_urbanization_lod_generate(&mut app);
	app.insert_resource(UrbanizationModeConfig::<StreamMode, Richmond<OnTerrain<Durham>>>::new(
		RichmondConfig::world_defaults(),
	));
	app.init_resource::<UrbanizationStreamKey>();
	app.init_resource::<UrbanizationLayerRegion>();
	app.init_resource::<DevelopmentEntryStore>();
	app.init_resource::<DevelopmentConfig>();
	app.add_systems(OnExit(ActiveGenerationMode::of::<StreamMode>()), |world: &mut World| {
		Richmond::<OnTerrain<Durham>>::clear_generation(world)
	});
	app.add_systems(OnExit(ActiveGenerationMode::of::<OtherMode>()), |world: &mut World| {
		Richmond::<OnTerrain<Durham>>::clear_generation(world)
	});
	app.world_mut().spawn((Camera3d::default(), Transform::from_xyz(0.0, 8.0, 0.0)));

	let bounds =
		Aabb3d::from_min_max(Vec3::new(4_000.0, 0.0, 4_000.0), Vec3::new(4_080.0, 1.0, 4_080.0));
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
		.run_system_once(stream_urbanization::<StreamMode, OnTerrain<Durham>>)
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

#[test]
fn different_budgets_build_and_apply_on_enter() -> anyhow::Result<()> {
	let mut app = App::new();
	app.add_plugins((
		MinimalPlugins,
		AssetPlugin::default(),
		StatesPlugin,
		GenerationModePlugin::<StreamMode>::initial(),
		GenerationModePlugin::<OtherMode>::default(),
		BaseTerrainGenerationCore::<Durham>::default(),
		UrbanizationGenerationPlugin::<StreamMode, Richmond<OnTerrain<Durham>>>::new(
			RichmondConfig::world_defaults(),
		),
		UrbanizationGenerationPlugin::<OtherMode, Richmond<OnTerrain<Durham>>>::new(
			RichmondConfig { generate_budget: 8, ..RichmondConfig::shared_world() },
		),
	));
	app.insert_resource(TerrainStreaming::<Durham>::new(false));
	app.finish();
	Richmond::<OnTerrain<Durham>>::apply_generation(
		app.world_mut(),
		&RichmondConfig::world_defaults(),
	);
	anyhow::ensure!(
		app.world().resource::<LodGenerateBudget<UrbanizationLodChan>>().ids_per_frame == 16,
		"initial generate budget"
	);
	anyhow::ensure!(
		app.world().resource::<DevelopmentConfig>().likelihood == PLAYGROUND_LIKELIHOOD,
		"initial development config"
	);

	Richmond::<OnTerrain<Durham>>::apply_generation(
		app.world_mut(),
		&RichmondConfig { generate_budget: 8, ..RichmondConfig::shared_world() },
	);
	anyhow::ensure!(
		app.world().resource::<LodGenerateBudget<UrbanizationLodChan>>().ids_per_frame == 8,
		"other mode budget"
	);
	Ok(())
}

#[test]
fn plugin_order_does_not_matter() -> anyhow::Result<()> {
	let mut app = App::new();
	app.add_plugins((
		MinimalPlugins,
		AssetPlugin::default(),
		StatesPlugin,
		UrbanizationGenerationPlugin::<OtherMode, Richmond<OnTerrain<Durham>>>::new(
			RichmondConfig::shared_world(),
		),
		UrbanizationGenerationPlugin::<StreamMode, Richmond<OnTerrain<Durham>>>::new(
			RichmondConfig::world_defaults(),
		),
		BaseTerrainGenerationCore::<Durham>::default(),
		GenerationModePlugin::<StreamMode>::initial(),
		GenerationModePlugin::<OtherMode>::default(),
	));
	app.insert_resource(TerrainStreaming::<Durham>::new(false));
	app.finish();
	anyhow::ensure!(
		app.is_plugin_added::<UrbanizationGenerationCore<Richmond<OnTerrain<Durham>>>>(),
		"core is installed"
	);
	Ok(())
}

#[test]
fn raw_cell_hands_its_floor_to_a_cooked_padded_replacement() -> anyhow::Result<()> {
	let id = Id::from_cell(Aabb3d::from_min_max(Vec3::ZERO, Vec3::ONE));
	let mut world = World::new();
	world.insert_resource(UrbanizationPaddedTerrainState {
		wanted: HashSet::from([id]),
		replaced: HashSet::new(),
	});
	let raw = world.spawn((PresentedTerrainScene(id), Visibility::Inherited)).id();
	let padded = world.spawn((PresentedPaddedTerrainScene(id), TerrainColliderMeshSource)).id();
	let run = |world: &mut World| {
		world
			.run_system_once(sync_raw_terrain_replacements)
			.map_err(|error| anyhow::anyhow!("{error:?}"))
	};

	run(&mut world)?;
	anyhow::ensure!(
		world.get::<TerrainSuperseded>(raw).is_none(),
		"uncooked pads cannot bear weight"
	);
	anyhow::ensure!(world.get::<Visibility>(raw) == Some(&Visibility::Inherited));

	world.entity_mut(padded).insert(TerrainTrimeshCollider);
	run(&mut world)?;
	anyhow::ensure!(world.get::<TerrainSuperseded>(raw).is_some(), "raw floor stays under pads");
	anyhow::ensure!(world.get::<Visibility>(raw) == Some(&Visibility::Hidden));

	world.resource_mut::<UrbanizationPaddedTerrainState>().wanted.clear();
	run(&mut world)?;
	anyhow::ensure!(world.get::<TerrainSuperseded>(raw).is_none(), "raw cell collides again");
	anyhow::ensure!(world.get::<Visibility>(raw) == Some(&Visibility::Inherited));
	Ok(())
}

#[test]
fn raw_cells_another_owner_superseded_stay_superseded() -> anyhow::Result<()> {
	let id = Id::from_cell(Aabb3d::from_min_max(Vec3::ZERO, Vec3::ONE));
	let mut world = World::new();
	world.init_resource::<UrbanizationPaddedTerrainState>();
	let raw = world
		.spawn((PresentedTerrainScene(id), Visibility::Hidden, TerrainSuperseded))
		.id();
	world
		.run_system_once(sync_raw_terrain_replacements)
		.map_err(|error| anyhow::anyhow!("{error:?}"))?;
	anyhow::ensure!(world.get::<TerrainSuperseded>(raw).is_some());
	anyhow::ensure!(world.get::<Visibility>(raw) == Some(&Visibility::Hidden));
	Ok(())
}

#[test]
fn padded_ready_does_not_claim_another_owners_raw_cell() -> anyhow::Result<()> {
	let id = Id::from_cell(Aabb3d::from_min_max(Vec3::ZERO, Vec3::ONE));
	let mut world = World::new();
	world.insert_resource(UrbanizationPaddedTerrainState {
		wanted: HashSet::from([id]),
		replaced: HashSet::new(),
	});
	let raw = world
		.spawn((PresentedTerrainScene(id), Visibility::Hidden, TerrainSuperseded))
		.id();
	world.spawn((
		PresentedPaddedTerrainScene(id),
		TerrainColliderMeshSource,
		TerrainTrimeshCollider,
	));
	let run = |world: &mut World| {
		world
			.run_system_once(sync_raw_terrain_replacements)
			.map_err(|error| anyhow::anyhow!("{error:?}"))
	};

	run(&mut world)?;
	anyhow::ensure!(world.get::<TerrainSuperseded>(raw).is_some(), "another owner's hide stays");
	anyhow::ensure!(
		!world.resource::<UrbanizationPaddedTerrainState>().replaced.contains(&id),
		"ready must not claim a cell this stream did not hide"
	);

	world.resource_mut::<UrbanizationPaddedTerrainState>().wanted.clear();
	run(&mut world)?;
	anyhow::ensure!(
		world.get::<TerrainSuperseded>(raw).is_some(),
		"releasing a cell we never owned must not unhide it"
	);
	anyhow::ensure!(world.get::<Visibility>(raw) == Some(&Visibility::Hidden));
	Ok(())
}

#[test]
fn padded_viewer_quant_is_stable_inside_cell() -> anyhow::Result<()> {
	anyhow::ensure!(quantize_viewer_xz_for_test(Vec3::new(0.1, 12.0, 7.9)) == (0, 0));
	anyhow::ensure!(quantize_viewer_xz_for_test(Vec3::new(8.0, 0.0, -0.1)) == (1, -1));
	Ok(())
}

fn subscribed_app() -> App {
	use layer_stack::subscribe_mode;

	let mut app = App::new();
	app.add_plugins((
		MinimalPlugins,
		StatesPlugin,
		GenerationModePlugin::<TestMode>::initial(),
		GenerationModePlugin::<OtherMode>::default(),
	));
	subscribe_mode::<(Urbanized, UrbanizationHosts), TestMode>(&mut app);
	subscribe_mode::<(Urbanized, PaddedCells), TestMode>(&mut app);
	app
}

#[test]
fn streamed_hosts_leave_when_the_layer_region_is_gone() -> anyhow::Result<()> {
	let mut app = subscribed_app();
	app.insert_resource(UrbanizationLayerRegion::default());
	app.insert_resource(playable_world_cell_layout());
	app.insert_resource(TerrainExtent::<Durham>::streamed(
		playable_world_cell_layout().presentation_region(),
	));
	app.insert_resource(TerrainEntryStore::default());
	app.insert_resource(WorldBaseTerrain(BaseTerrainNoise::from_config(&TerrainConfig::new(42))));
	app.insert_resource(UrbanizationIndex::default());
	app.insert_resource(DevelopmentEntryStore::default());
	app.init_resource::<UrbanizationPresenterState>();
	app.init_resource::<lod::LodPresentGate<(Urbanized, UrbanizationHosts)>>();

	let id = Id::from_cell(Aabb3d::from_min_max(Vec3::ZERO, Vec3::ONE));
	let host = app.world_mut().spawn_empty().id();
	app.world_mut()
		.resource_mut::<UrbanizationPresenterState>()
		.insert_presented_for_test(id, vec![host]);
	anyhow::ensure!(
		app.world().resource::<UrbanizationPresenterState>().presented_ids() == [id],
		"seed host"
	);

	app.world_mut()
		.run_system_once(present_richmond_hosts::<OnTerrain<Durham>>)
		.map_err(|error| anyhow::anyhow!("{error:?}"))?;
	anyhow::ensure!(
		app.world().resource::<UrbanizationPresenterState>().presented_ids().is_empty(),
		"streamed layout with no layer region clears hosts"
	);
	anyhow::ensure!(app.world().get_entity(host).is_err(), "host entity leaves");
	Ok(())
}

#[test]
fn hosts_walk_a_stored_development_with_no_hopscotch() -> anyhow::Result<()> {
	let mut world = World::new();
	world.insert_resource(UrbanizationLayerRegion::default());
	let layout = fine_patch_cell_layout(2, bevy::math::IVec2::ZERO);
	world.insert_resource(layout.clone());
	world.insert_resource(TerrainEntryStore::default());
	world.insert_resource(WorldBaseTerrain(BaseTerrainNoise::from_config(&TerrainConfig::new(42))));
	world.insert_resource(UrbanizationIndex::default());
	world.insert_resource(DevelopmentEntryStore::default());

	let cell = Aabb3d::from_min_max(Vec3::new(-20.0, 0.0, -20.0), Vec3::new(20.0, 1.0, 20.0));
	let config = DevelopmentConfig::from_world_seed(42);
	let filled = DevelopmentCell::with_les_halles(cell, 12.0, &config);
	let built = filled
		.built(config.seed as i32)
		.ok_or_else(|| anyhow::anyhow!("les halles built"))?;
	let id = Id::from_cell(filled.cell);
	{
		let mut store = world.resource_mut::<DevelopmentEntryStore>();
		store.insert_cell(id, filled);
		store.insert_built(id, built, cell);
	}

	let region = urbanization_host_region(
		&TerrainExtent::<Durham>::pinned(layout.presentation_region()),
		None,
	)
	.ok_or_else(|| anyhow::anyhow!("fine-patch host region"))?;
	let mut state = SystemState::<TerrainView<Urbanized>>::new(&mut world);
	let view = state.get(&world).map_err(|error| anyhow::anyhow!("{error:?}"))?;
	let ids: Vec<_> = Urbanized::built_overlapping(&view.read, region)
		.into_iter()
		.map(|(id, _, _)| id)
		.collect();
	drop(state);
	anyhow::ensure!(
		ids.contains(&id),
		"hosts walk a stored development with no hopscotch selection"
	);
	anyhow::ensure!(
		world
			.resource::<UrbanizationIndex>()
			.filled_leaves_overlapping(region)
			.is_empty(),
		"no hopscotch cells are selected"
	);
	Ok(())
}
