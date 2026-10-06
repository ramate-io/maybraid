use std::collections::HashSet;

use bevy::app::App;
use bevy::ecs::system::{RunSystemOnce, SystemState};
use bevy::math::bounding::Aabb3d;
use bevy::math::{Vec2, Vec3};
use bevy::prelude::{
	AssetPlugin, Camera3d, MinimalPlugins, NextState, OnExit, Transform, Visibility, World,
};
use bevy::state::app::StatesPlugin;
use durham::{
	fine_patch_cell_layout, playable_world_cell_layout, BaseTerrainNoise, Durham,
	DurhamTerrainConfig, HcsgStorage, PresentedTerrainScene, TerrainCellLayout,
	TerrainColliderMeshSource, TerrainConfig, TerrainStorage, TerrainSuperseded,
	TerrainTrimeshCollider, WorldBaseTerrain, TERRAIN_CELL_SIZE,
};
use layer_stack::{
	ActiveGenerationMode, Generate, GenerationMode, GenerationModePlugin, LayerGenerationCore,
	LayerModeConfig, Present, Scheme,
};
use lod::gen::{Id, LodGenerateBudget, OriginalId};
use lod::hcsg::{universal_bounds, CurrentBounds, GenerateOn, GenerationProducer, Seed};
use procedural_common::noise_params_from_scalar_str;
use terrain_layer_model::{HeightField, OnTerrain, TerrainExtent, TerrainStreaming, TerrainView};
use urbanization_cells::{
	register_urbanization_nodes, SelectedUrbanization, UrbanDevelopmentKind, UrbanizationExtent,
	UrbanizationKind, UrbanizationWindow,
};
use urbanization_layer_model::{
	urbanization_host_region, UrbanModel, UrbanSnapshot, Urbanization, UrbanizationGeneration,
	UrbanizationLayerRegion,
};
use urbanization_layer_presentation::{PaddedCells, UrbanizationHosts};

use crate::built::Built;
use crate::layer::Richmond;
use crate::layer_config::{
	DevelopmentFocus, RichmondConfig, UrbanizationStreamSpec, DEFAULT_URBANIZATION_NOISE,
	DEFAULT_URBANIZATION_STREAM_RADIUS, PLAYGROUND_LIKELIHOOD,
};
use crate::layer_present::{
	present_richmond_hosts, quantize_viewer_xz_for_test, sync_raw_terrain_replacements,
	UrbanizationPaddedTerrainState, UrbanizationPresenterState,
};
use crate::layer_stream::{parse_urbanization_kind, stream_radii_m, stream_urbanization, HostWindow};
use crate::padded::{PaddedTerrain, PresentedPaddedTerrainScene, TerrainWithPads};
use crate::storage::{register_richmond_nodes, RichmondNodes};
use crate::{
	AuthoredDevelopment, AuthoredDevelopments, DevelopmentConfig, DevelopmentKind, DevelopmentSite,
	DevelopmentSites,
	PadComplex, PadParams, DEVELOPMENT_CELL_SIZE,
};

struct TestMode;

impl GenerationMode for TestMode {}

impl Scheme<OnTerrain<Durham>> for TestMode {
	fn install(_app: &mut App, _config: &DurhamTerrainConfig) {}
}

impl Scheme<Urbanization<Richmond<OnTerrain<Durham>>>> for TestMode {
	fn install(_app: &mut App, _config: &RichmondConfig) {}
}

struct StreamMode;
struct OtherMode;

impl GenerationMode for StreamMode {}
impl GenerationMode for OtherMode {}

impl Scheme<OnTerrain<Durham>> for StreamMode {
	fn install(_app: &mut App, _config: &DurhamTerrainConfig) {}
}

impl Scheme<OnTerrain<Durham>> for OtherMode {
	fn install(_app: &mut App, _config: &DurhamTerrainConfig) {}
}

impl Scheme<Urbanization<Richmond<OnTerrain<Durham>>>> for StreamMode {
	fn install(_app: &mut App, _config: &RichmondConfig) {}
}

impl Scheme<Urbanization<Richmond<OnTerrain<Durham>>>> for OtherMode {
	fn install(_app: &mut App, _config: &RichmondConfig) {}
}

type Ground = OnTerrain<Durham>;
type Urbanized = Urbanization<Richmond<Ground>>;

fn registered_storage() -> HcsgStorage {
	let mut storage = HcsgStorage::default();
	register_urbanization_nodes(&mut storage);
	register_richmond_nodes::<Ground>(&mut storage);
	storage
}

fn empty_urbanized_world() -> World {
	let mut world = World::new();
	world.insert_resource(registered_storage());
	world.insert_resource(TerrainCellLayout::default());
	world.insert_resource(WorldBaseTerrain(BaseTerrainNoise::from_config(&TerrainConfig::new(42))));
	world
}

fn authored_only_config() -> DevelopmentConfig {
	DevelopmentConfig {
		sites: DevelopmentSites::Authored,
		..DevelopmentConfig::from_world_seed(42)
	}
}

fn les_halles_at(center: Vec2, config: &DevelopmentConfig) -> AuthoredDevelopment {
	let half = DEVELOPMENT_CELL_SIZE * 0.5;
	AuthoredDevelopment {
		cell: Aabb3d::from_min_max(
			Vec3::new(center.x - half, 0.0, center.y - half),
			Vec3::new(center.x + half, 1.0, center.y + half),
		),
		kinds: vec![DevelopmentKind::LesHalles],
		height: 12.0,
		config: config.clone(),
		courtyard: None,
	}
}

fn generated<T>(storage: &mut HcsgStorage, id: Id) -> Option<&T>
where
	T: lod::gen::GenerationScheme<HcsgStorage> + Send + Sync + 'static,
{
	storage.get_or_generate::<T>(id)?;
	storage.get::<T>(id)
}

fn seed_roots(storage: &mut HcsgStorage, config: DevelopmentConfig, authored: AuthoredDevelopments) {
	storage.seed(config, universal_bounds());
	storage.seed(authored, universal_bounds());
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
fn selection_applies_focus_when_the_spec_kind_is_open() -> anyhow::Result<()> {
	let mut config = RichmondConfig::world_defaults();
	config.focus_urbanization = Some(UrbanizationKind::Frontier);
	anyhow::ensure!(
		config.urbanization.as_ref().is_some_and(|spec| spec.kind.is_none()),
		"world defaults leave kind open"
	);
	anyhow::ensure!(
		config.urbanization_selection().kind == Some(UrbanizationKind::Frontier),
		"the selection must keep the focused kind"
	);
	anyhow::ensure!(
		config.development_config().sites == DevelopmentSites::Urbanization,
		"a spec places on leaves"
	);
	Ok(())
}

#[test]
fn a_mode_without_a_stream_or_focus_places_only_authored_developments() -> anyhow::Result<()> {
	anyhow::ensure!(
		RichmondConfig::shared_world().development_config().sites == DevelopmentSites::Authored
	);
	let catalog = RichmondConfig {
		focus_development: Some(DevelopmentFocus::LesHalles),
		..RichmondConfig::shared_world()
	};
	anyhow::ensure!(catalog.development_config().sites == DevelopmentSites::Lattice);
	Ok(())
}

#[test]
fn urbanization_stack_names_the_local_mode() {
	let _stack = (
		GenerationModePlugin::<TestMode>::initial(),
		Generate::<TestMode, OnTerrain<Durham>>::new(DurhamTerrainConfig::fine_patch(2)),
		Generate::<TestMode, Urbanization<Richmond<OnTerrain<Durham>>>>::new(
			RichmondConfig::default(),
		),
		Present::<TestMode, Urbanization<Richmond<OnTerrain<Durham>>>>::default(),
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

fn insert_overlay_pad(
	world: &mut World,
	source: Id,
	bounds: Aabb3d,
	res_2: u8,
) -> anyhow::Result<()> {
	use terrain_layer_model::TerrainCell;

	let mut storage = world.resource_mut::<HcsgStorage>();
	let mut padded = {
		let terrain = storage.terrain(source).ok_or_else(|| anyhow::anyhow!("source terrain"))?;
		TerrainWithPads::compose(terrain, std::iter::empty::<&PadComplex>())
	};
	padded.cell = bounds;
	padded.res_2 = res_2;
	let bounds = padded.bounds();
	storage.insert(Id::from_cell(bounds), PaddedTerrain::<Ground>::new(padded), bounds);
	Ok(())
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
	let base = BaseTerrainNoise::from_config(&TerrainConfig::new(42));
	let fine = TerrainCellLayout::default();
	let medium =
		TerrainCellLayout { cell_size: 2.0 * TERRAIN_CELL_SIZE, ..TerrainCellLayout::default() };
	{
		let mut store = world.resource_mut::<HcsgStorage>();
		store.insert_base_terrain_for_test(&fine, 0, 0, base.clone());
		store.insert_base_terrain_for_test(&medium, 0, 0, base);
	}
	let query = Aabb3d::from_min_max(Vec3::new(1.0, -10.0, 1.0), Vec3::new(20.0, 10.0, 20.0));
	let (source, fine_bounds, medium_bounds) = {
		let store = world.resource::<HcsgStorage>();
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
	let clear = |world: &mut World| world.resource_mut::<HcsgStorage>().clear_group::<RichmondNodes>();

	insert_overlay_pad(&mut world, source, medium_bounds, 9)?;
	let (width, res) = overlay_width_res(&mut world, query, TERRAIN_CELL_SIZE, None)?;
	anyhow::ensure!((width - 2.0 * TERRAIN_CELL_SIZE).abs() < 1e-3);
	anyhow::ensure!(res == 9);

	clear(&mut world);
	let (width, res) = overlay_width_res(&mut world, query, TERRAIN_CELL_SIZE, None)?;
	anyhow::ensure!((width - TERRAIN_CELL_SIZE).abs() < 1e-3);
	anyhow::ensure!(res == 0);

	insert_overlay_pad(&mut world, source, fine_bounds, 9)?;
	let (width, res) = overlay_width_res(&mut world, query, 2.0 * TERRAIN_CELL_SIZE, Some(1e-2))?;
	anyhow::ensure!((width - 2.0 * TERRAIN_CELL_SIZE).abs() < 1e-3);
	anyhow::ensure!(res == 0);

	clear(&mut world);
	insert_overlay_pad(&mut world, source, medium_bounds, 9)?;
	let (width, res) = overlay_width_res(&mut world, query, 2.0 * TERRAIN_CELL_SIZE, Some(1e-2))?;
	anyhow::ensure!((width - 2.0 * TERRAIN_CELL_SIZE).abs() < 1e-3);
	anyhow::ensure!(res == 9);
	Ok(())
}

fn stream_app<Mode: GenerationMode>(config: RichmondConfig) -> App {
	let mut app = App::new();
	app.add_plugins((MinimalPlugins, StatesPlugin));
	app.add_plugins((
		GenerationModePlugin::<StreamMode>::initial(),
		GenerationModePlugin::<OtherMode>::default(),
		GenerateOn::<UrbanizationWindow, SelectedUrbanization>::default(),
	));
	let mut storage = registered_storage();
	storage.seed(config.urbanization_selection(), universal_bounds());
	seed_roots(&mut storage, config.development_config(), AuthoredDevelopments::default());
	app.insert_resource(storage);
	app.insert_resource(LayerModeConfig::<Mode, Urbanized>::new(config));
	app.init_resource::<UrbanizationLayerRegion>();
	app.world_mut().spawn((Camera3d::default(), Transform::from_xyz(0.0, 8.0, 0.0)));
	app
}

fn run_stream<Mode: GenerationMode>(app: &mut App) -> anyhow::Result<()> {
	app.world_mut()
		.run_system_once(stream_urbanization::<Mode, Ground>)
		.map_err(|error| anyhow::anyhow!("{error:?}"))
}

#[test]
fn stream_layer_region_covers_every_leaf_of_the_selected_cells() -> anyhow::Result<()> {
	let mut config = RichmondConfig::world_defaults();
	config.focus_urbanization = Some(UrbanizationKind::MixedAgeCity);
	let mut app = stream_app::<StreamMode>(config);
	run_stream::<StreamMode>(&mut app)?;

	let host = app
		.world()
		.resource::<UrbanizationLayerRegion>()
		.region
		.ok_or_else(|| anyhow::anyhow!("layer region"))?;
	anyhow::ensure!(
		app.world().resource::<CurrentBounds<UrbanizationWindow>>().bounds.is_some(),
		"the stream publishes the urbanization window"
	);

	let extents = UrbanizationExtent::cells_overlapping(host);
	anyhow::ensure!(!extents.is_empty(), "the layer region holds urbanization cells");
	let mut storage = app.world_mut().resource_mut::<HcsgStorage>();
	let mut filled = 0;
	for extent in extents {
		let Some(selected) = generated::<SelectedUrbanization>(&mut storage, extent.id()) else {
			continue;
		};
		for leaf in selected.leaves.iter().filter(|leaf| leaf.kind != UrbanDevelopmentKind::Empty) {
			filled += 1;
			anyhow::ensure!(
				leaf.bounds.min.x >= host.min.x
					&& leaf.bounds.min.z >= host.min.z
					&& leaf.bounds.max.x <= host.max.x
					&& leaf.bounds.max.z <= host.max.z,
				"leaf {:?} sits outside the layer region",
				leaf.bounds
			);
		}
	}
	anyhow::ensure!(filled > 0, "hopscotch produced no filled leaves");
	Ok(())
}

#[test]
fn leaving_a_stream_mode_clears_then_reentering_streams_again() -> anyhow::Result<()> {
	let mut config = RichmondConfig::world_defaults();
	config.focus_urbanization = Some(UrbanizationKind::MixedAgeCity);
	let mut app = stream_app::<StreamMode>(config);
	app.add_systems(OnExit(ActiveGenerationMode::of::<StreamMode>()), |world: &mut World| {
		Richmond::<Ground>::clear_generation(world);
	});
	app.add_systems(OnExit(ActiveGenerationMode::of::<OtherMode>()), |world: &mut World| {
		Richmond::<Ground>::clear_generation(world);
	});

	let extent = UrbanizationExtent::default_cell();
	let site = {
		let mut storage = app.world_mut().resource_mut::<HcsgStorage>();
		anyhow::ensure!(storage.get_or_generate::<SelectedUrbanization>(extent.id()).is_some());
		let OriginalId(site) = storage
			.original_ids_for::<DevelopmentSite>(extent.aabb())
			.first()
			.copied()
			.ok_or_else(|| anyhow::anyhow!("a development site"))?;
		anyhow::ensure!(storage.get_or_generate::<DevelopmentSite>(site).is_some());
		site
	};
	app.update();
	run_stream::<StreamMode>(&mut app)?;
	anyhow::ensure!(app.world().resource::<UrbanizationLayerRegion>().region.is_some());

	app.world_mut()
		.resource_mut::<NextState<ActiveGenerationMode>>()
		.set(ActiveGenerationMode::of::<OtherMode>());
	app.update();
	let storage = app.world().resource::<HcsgStorage>();
	anyhow::ensure!(
		storage.get::<DevelopmentSite>(site).is_none(),
		"the other mode holds no streamed development"
	);
	anyhow::ensure!(
		storage.get::<SelectedUrbanization>(extent.id()).is_none(),
		"the other mode holds no hopscotch selection"
	);
	anyhow::ensure!(app.world().resource::<UrbanizationLayerRegion>().region.is_none());
	anyhow::ensure!(
		app.world().resource::<CurrentBounds<UrbanizationWindow>>().bounds.is_none(),
		"leaving forgets the urbanization window"
	);

	app.world_mut()
		.resource_mut::<NextState<ActiveGenerationMode>>()
		.set(ActiveGenerationMode::of::<StreamMode>());
	app.update();
	run_stream::<StreamMode>(&mut app)?;
	anyhow::ensure!(
		app.world().resource::<CurrentBounds<UrbanizationWindow>>().bounds.is_some(),
		"re-entering publishes the urbanization window again"
	);
	anyhow::ensure!(app.world().resource::<UrbanizationLayerRegion>().region.is_some());
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
		LayerGenerationCore::<OnTerrain<Durham>>::default(),
		Generate::<StreamMode, Urbanization<Richmond<OnTerrain<Durham>>>>::new(
			RichmondConfig::world_defaults(),
		),
		Generate::<OtherMode, Urbanization<Richmond<OnTerrain<Durham>>>>::new(RichmondConfig {
			generate_budget: 8,
			..RichmondConfig::shared_world()
		}),
	));
	app.insert_resource(TerrainStreaming::<Durham>::new(false));
	app.finish();
	Richmond::<OnTerrain<Durham>>::apply_generation(
		app.world_mut(),
		&RichmondConfig::world_defaults(),
	);
	anyhow::ensure!(
		app.world().resource::<LodGenerateBudget<UrbanizationWindow>>().ids_per_frame == 16,
		"initial generate budget"
	);
	anyhow::ensure!(
		app.world().resource::<LodGenerateBudget<HostWindow>>().ids_per_frame == 16,
		"initial host budget"
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
		app.world().resource::<LodGenerateBudget<UrbanizationWindow>>().ids_per_frame == 8,
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
		Generate::<OtherMode, Urbanization<Richmond<OnTerrain<Durham>>>>::new(
			RichmondConfig::shared_world(),
		),
		Generate::<StreamMode, Urbanization<Richmond<OnTerrain<Durham>>>>::new(
			RichmondConfig::world_defaults(),
		),
		LayerGenerationCore::<OnTerrain<Durham>>::default(),
		GenerationModePlugin::<StreamMode>::initial(),
		GenerationModePlugin::<OtherMode>::default(),
	));
	app.insert_resource(TerrainStreaming::<Durham>::new(false));
	app.finish();
	anyhow::ensure!(
		app.is_plugin_added::<LayerGenerationCore<Urbanization<Richmond<OnTerrain<Durham>>>>>(),
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
	app.insert_resource(registered_storage());
	app.insert_resource(WorldBaseTerrain(BaseTerrainNoise::from_config(&TerrainConfig::new(42))));
	app.init_resource::<UrbanizationPresenterState>();
	app.init_resource::<lod::LodPresentGate<Urbanized>>();

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
		.run_system_once(present_richmond_hosts::<Ground>)
		.map_err(|error| anyhow::anyhow!("{error:?}"))?;
	anyhow::ensure!(
		app.world().resource::<UrbanizationPresenterState>().presented_ids().is_empty(),
		"streamed layout with no layer region clears hosts"
	);
	anyhow::ensure!(app.world().get_entity(host).is_err(), "host entity leaves");
	Ok(())
}

#[test]
fn hosts_walk_an_authored_development_with_no_hopscotch() -> anyhow::Result<()> {
	let mut world = empty_urbanized_world();
	world.insert_resource(UrbanizationLayerRegion::default());
	let layout = fine_patch_cell_layout(2, bevy::math::IVec2::ZERO);
	world.insert_resource(layout.clone());

	let config = authored_only_config();
	let authored = les_halles_at(Vec2::ZERO, &config);
	let id = authored.id();
	let region = urbanization_host_region(
		&TerrainExtent::<Durham>::pinned(layout.presentation_region()),
		None,
	)
	.ok_or_else(|| anyhow::anyhow!("fine-patch host region"))?;
	{
		let mut storage = world.resource_mut::<HcsgStorage>();
		seed_roots(&mut storage, config, AuthoredDevelopments(vec![authored]));
		for OriginalId(origin) in storage.original_ids_for::<Built<Ground>>(region) {
			storage.get_or_generate::<Built<Ground>>(origin);
		}
	}

	let mut state = SystemState::<TerrainView<Urbanized>>::new(&mut world);
	let view = state.get(&world).map_err(|error| anyhow::anyhow!("{error:?}"))?;
	let ids: Vec<_> = Urbanized::built_overlapping(&view.read, region)
		.into_iter()
		.map(|(id, _, _)| id)
		.collect();
	drop(state);
	anyhow::ensure!(ids == [id], "hosts walk only the authored development, got {ids:?}");
	anyhow::ensure!(
		world.resource::<HcsgStorage>().overlapping::<SelectedUrbanization>(region).is_empty(),
		"no hopscotch cells are selected"
	);
	Ok(())
}

#[test]
fn padded_terrain_follows_the_developments_over_its_ground_cell() -> anyhow::Result<()> {
	let base = BaseTerrainNoise::from_config(&TerrainConfig::new(42));
	let layout = TerrainCellLayout::default();
	let mut storage = registered_storage();
	storage.insert_base_terrain_for_test(&layout, 0, 0, base.clone());
	storage.insert_base_terrain_for_test(&layout, 40, 40, base.clone());
	let ids = |storage: &HcsgStorage, ix: f32, iz: f32| {
		let center = Vec3::new((ix + 0.5) * layout.cell_size, 0.0, (iz + 0.5) * layout.cell_size);
		storage.terrain_ids_overlapping(Aabb3d::new(center, Vec3::splat(0.25)))
	};
	let under = ids(&storage, 0.0, 0.0).first().copied().ok_or_else(|| anyhow::anyhow!("near"))?;
	let far = ids(&storage, 40.0, 40.0).first().copied().ok_or_else(|| anyhow::anyhow!("far"))?;
	let under_center = storage
		.terrain(under)
		.map(|terrain| Vec2::new(terrain.cell.min.x + terrain.cell.max.x, terrain.cell.min.z + terrain.cell.max.z) * 0.5)
		.ok_or_else(|| anyhow::anyhow!("near terrain"))?;

	let config = authored_only_config();
	let authored = les_halles_at(under_center, &config);
	seed_roots(&mut storage, config, AuthoredDevelopments(vec![authored]));

	let padded = generated::<PaddedTerrain<Ground>>(&mut storage, under)
		.ok_or_else(|| anyhow::anyhow!("padded terrain under the development"))?;
	anyhow::ensure!(padded.surface.pad_count > 0, "the development's pads are composed in");
	anyhow::ensure!(
		storage.get_or_generate::<PaddedTerrain<Ground>>(far).is_none(),
		"a cell no pad reaches stays raw"
	);

	let version = storage.entry::<PaddedTerrain<Ground>>(under).map(|entry| entry.version);
	storage.insert_base_terrain_for_test(&layout, 80, 80, base);
	anyhow::ensure!(
		storage.entry::<PaddedTerrain<Ground>>(under).map(|entry| entry.version) == version,
		"an unrelated ground write leaves the padded cell alone"
	);
	Ok(())
}

#[test]
fn reseeding_authored_developments_regenerates_a_stationary_window() -> anyhow::Result<()> {
	let config = authored_only_config();
	let authored = les_halles_at(Vec2::ZERO, &config);
	let id = authored.id();

	let mut app = App::new();
	app.add_plugins(MinimalPlugins);
	app.insert_resource(registered_storage());
	app.insert_resource(config);
	app.insert_resource(AuthoredDevelopments::default());
	app.add_plugins((
		Seed::<DevelopmentConfig>::default().invalidates::<RichmondNodes>().restarts::<HostWindow>(),
		Seed::<AuthoredDevelopments>::default()
			.invalidates::<RichmondNodes>()
			.restarts::<HostWindow>(),
		GenerateOn::<HostWindow, Built<Ground>>::default(),
	));
	let keep = Aabb3d::new(Vec3::ZERO, Vec3::new(400.0, 1.0, 400.0));
	app.world_mut()
		.run_system_once(move |mut hosts: GenerationProducer<HostWindow>| {
			hosts.publish(keep, None);
		})
		.map_err(|error| anyhow::anyhow!("{error:?}"))?;
	let built = |app: &App| app.world().resource::<HcsgStorage>().get::<Built<Ground>>(id).is_some();

	for _ in 0..8 {
		app.update();
	}
	anyhow::ensure!(!built(&app), "nothing is authored yet");

	app.world_mut().resource_mut::<AuthoredDevelopments>().0.push(authored);
	for _ in 0..8 {
		app.update();
		if built(&app) {
			return Ok(());
		}
	}
	anyhow::bail!("the reseed regenerates the unmoved host window")
}
