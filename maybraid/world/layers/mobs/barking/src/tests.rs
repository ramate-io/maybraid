use bevy::app::{App, Plugin};
use bevy::ecs::system::{Res, RunSystemOnce, SystemParamItem};
use bevy::math::bounding::Aabb3d;
use bevy::math::{Vec2, Vec3, Vec3Swizzles};
use bevy::prelude::{GlobalTransform, Transform, World};
use chico::{Chico, ForestIndex};
use durham::{
	BaseTerrainNoise, Durham, TerrainCellLayout, TerrainConfig, TerrainEntryStore, WorldBaseTerrain,
};
use layer_stack::{Generate, LayerGenerationCore, RequireLayer, Scheme};
use lod::gen::{GenerationScheme, Id, LodGenerateKeepRegion, SpatialIndex};
use lod::lod_ref::LodRef;
use lod::presentation::LodPresentKeepRegion;
use procedural_common::NoiseParams;
use richmond::{DevelopmentConfig, DevelopmentEntryStore};
use terrain_layer_model::{HeightField, OnTerrain, TerrainCell, TerrainModel};
use urbanization_cells::{
	DevelopmentLeaf, SelectedUrbanization, UrbanDevelopmentKind, UrbanizationExtent,
	UrbanizationIndex, UrbanizationKind,
};
use urbanization_layer_model::{UrbanSetting, Urbanization};
use vegetation_layer_model::{Vegetation, VegetationGeneration, VegetationModel};

use crate::generation::{GroupKind, MobGroup, MobPlantHost, MobWorldSample};
use crate::index::{urban_leaf_arrival_radius, MobCell, MobCellExtent, MobIndex};
use crate::sample::{
	DiscoverablePlaces, ForestSelection, PlantHosts, SelectUrbanization, UrbanSelection,
};
use crate::stream::{
	stream_mob_generate, stream_mob_present, sync_mob_models, sync_mob_plant_hosts,
	MobGenerateBullseye, MobLodChan, MobPresentBullseye,
};
use crate::{Barking, BarkingConfig};
use mob_layer_model::Mobs;

type Urbanized = Urbanization<richmond::Richmond<OnTerrain<Durham>>>;

pub(crate) fn insert_urbanized_resources(world: &mut World) {
	world.insert_resource(TerrainEntryStore::default());
	world.insert_resource(TerrainCellLayout::default());
	world.insert_resource(WorldBaseTerrain(BaseTerrainNoise::from_config(&TerrainConfig::new(42))));
	world.insert_resource(DevelopmentEntryStore::default());
	world.insert_resource(UrbanizationIndex::default());
}

#[test]
fn mob_cells_cover_a_four_hundred_metre_lattice() {
	let region = Aabb3d::from_min_max(Vec3::new(-199.0, 0.0, -199.0), Vec3::new(201.0, 1.0, 201.0));
	assert_eq!(MobCellExtent::cells_overlapping(region).len(), 4);
}

#[test]
fn bounds_extent_keeps_the_given_rectangle() -> anyhow::Result<()> {
	let min = Vec3::new(-36.0, 0.0, -28.0);
	let max = Vec3::new(36.0, 1.0, 28.0);
	let extent = MobCellExtent::from_bounds(min, max);
	anyhow::ensure!(extent.aabb() == Aabb3d::from_min_max(min, max));
	anyhow::ensure!(MobCellExtent::from_id(extent.id()).is_none(), "grid from_id stays 400 m");
	Ok(())
}

#[test]
fn origin_cell_is_always_populated_when_models_are_ready() -> anyhow::Result<()> {
	let mut index = MobIndex::ready();
	let extent = MobCellExtent::from_cell_index(0, 0);
	let (cell, _) = MobCell::build_with_id(&mut index, extent.id())
		.ok_or_else(|| anyhow::anyhow!("origin mob cell did not generate"))?;
	assert!(!cell.groups.is_empty());
	Ok(())
}

#[test]
fn frontier_hosts_keep_urban_families_inside_the_arrival_disk() {
	let index = MobIndex::ready_frontier(vec![MobPlantHost {
		xz: Vec2::new(20.0, -8.0),
		arrival_radius: 6.0,
	}]);
	let group = MobGroup::generate(GroupKind::Frontier, 11, Vec2::ZERO, &index);
	let planted: Vec<_> = group
		.mobs
		.iter()
		.filter(|mob| {
			matches!(
				mob.scene.mob.kind,
				mob_scenes::MobKind::Guard
					| mob_scenes::MobKind::Brawler
					| mob_scenes::MobKind::Pleb
			)
		})
		.collect();
	assert!(!planted.is_empty());
	for mob in planted {
		let xz = Vec2::new(mob.transform.translation.x, mob.transform.translation.z);
		assert!(xz.distance(Vec2::new(20.0, -8.0)) <= 6.0 + 1e-4);
		assert_eq!(mob.transform.translation.y, 0.0);
	}
}

#[test]
fn mob_models_follow_a_late_urbanization_pin() -> anyhow::Result<()> {
	let mut app = App::new();
	insert_urbanized_resources(app.world_mut());
	app.init_resource::<ForestIndex>();
	app.init_resource::<MobIndex>();
	app.add_systems(bevy::prelude::Update, sync_mob_models::<Chico<Urbanized>, Urbanized>);
	app.update();
	let pinned = NoiseParams { frequency: 0.0005, ..Default::default() };
	app.world_mut().resource_mut::<UrbanizationIndex>().noise = pinned;
	app.update();
	let mobs = app
		.world()
		.get_resource::<MobIndex>()
		.ok_or_else(|| anyhow::anyhow!("mob index missing"))?;
	assert!(mobs.models_ready);
	assert_eq!(mobs.urbanization_noise, pinned);
	Ok(())
}

#[test]
fn urban_leaf_arrival_matches_setting_formula() {
	let bounds = Aabb3d::from_min_max(Vec3::new(-40.0, 0.0, -20.0), Vec3::new(40.0, 1.0, 20.0));
	assert_eq!(urban_leaf_arrival_radius(bounds), 10.0);
}

#[test]
fn plant_hosts_follow_leaves_cells_settings_then_places() -> anyhow::Result<()> {
	let mut app = App::new();
	insert_urbanized_resources(app.world_mut());
	app.init_resource::<ForestIndex>();
	app.insert_resource(MobIndex::ready());
	app.init_resource::<LodGenerateKeepRegion<MobLodChan>>();

	let leaf_bounds =
		Aabb3d::from_min_max(Vec3::new(-40.0, 0.0, -20.0), Vec3::new(40.0, 1.0, 20.0));
	let cell_bounds =
		Aabb3d::from_min_max(Vec3::new(80.0, 0.0, -20.0), Vec3::new(160.0, 1.0, 20.0));
	let region = Aabb3d::from_min_max(Vec3::new(-50.0, 0.0, -50.0), Vec3::new(900.0, 1.0, 50.0));
	app.world_mut().resource_mut::<LodGenerateKeepRegion<MobLodChan>>().region = Some(region);

	let extent = UrbanizationExtent::default_cell();
	let selected = SelectedUrbanization {
		extent,
		kind: UrbanizationKind::Frontier,
		leaves: vec![DevelopmentLeaf {
			bounds: leaf_bounds,
			kind: UrbanDevelopmentKind::LesHalles,
		}],
	};
	SpatialIndex::<SelectedUrbanization>::insert(
		&mut *app.world_mut().resource_mut::<UrbanizationIndex>(),
		extent.id(),
		selected,
		extent.aabb(),
	);

	let config = DevelopmentConfig::from_world_seed(42);
	app.world_mut().resource_mut::<DevelopmentEntryStore>().insert_cell(
		Id::from_cell(cell_bounds),
		richmond::DevelopmentCell::with_les_halles(cell_bounds, 12.0, &config),
	);

	let setting_at = Vec3::new(12.0, 4.0, -6.0);
	app.world_mut().spawn((
		UrbanSetting { id: Id::from_cell(leaf_bounds), arrival_radius: 14.0 },
		GlobalTransform::from_translation(setting_at),
	));
	let place_at = Vec3::new(-8.0, 2.0, 18.0);
	app.world_mut().spawn((
		richmond::DiscoverablePlace::host(richmond::DiscoverablePlaceLabel::Storey, 9.0, 1.1),
		GlobalTransform::from_translation(place_at),
	));

	app.add_systems(bevy::prelude::Update, sync_mob_plant_hosts::<Urbanized>);
	app.update();
	let hosts = app.world().resource::<MobIndex>().plant_hosts.clone();
	anyhow::ensure!(hosts.len() == 4, "expected four hosts, got {}", hosts.len());
	assert_eq!(
		hosts[0],
		MobPlantHost {
			xz: Vec2::new(
				(leaf_bounds.min.x + leaf_bounds.max.x) * 0.5,
				(leaf_bounds.min.z + leaf_bounds.max.z) * 0.5,
			),
			arrival_radius: urban_leaf_arrival_radius(leaf_bounds),
		}
	);
	assert_eq!(
		hosts[1],
		MobPlantHost {
			xz: Vec2::new(
				(cell_bounds.min.x + cell_bounds.max.x) * 0.5,
				(cell_bounds.min.z + cell_bounds.max.z) * 0.5,
			),
			arrival_radius: urban_leaf_arrival_radius(cell_bounds),
		}
	);
	assert_eq!(hosts[2], MobPlantHost { xz: setting_at.xz(), arrival_radius: 14.0 });
	assert_eq!(hosts[3], MobPlantHost { xz: place_at.xz(), arrival_radius: 9.0 });

	let urbanization = app.world().resource::<UrbanizationIndex>();
	for extent in UrbanizationExtent::cells_overlapping(region) {
		anyhow::ensure!(
			urbanization.get(extent.id()).is_some(),
			"generate keep cell {:?} was not selected",
			extent.id()
		);
	}
	Ok(())
}

#[test]
fn stub_sampling_drives_the_index() -> anyhow::Result<()> {
	let mut index = MobIndex::default();
	index.configure_from(
		NoiseParams { frequency: 0.25, ..Default::default() },
		None,
		NoiseParams::default(),
		Some(UrbanizationKind::Frontier),
		stub_layers_at,
		stub_kind_at,
	);
	let sample = index.sample_mobs(Vec2::ZERO);
	anyhow::ensure!((sample.vegetation - 1.0).abs() < 1e-5, "stub layers fill every slot");
	anyhow::ensure!((sample.urbanization - 0.4).abs() < 1e-5, "frontier weight");
	Ok(())
}

fn stub_layers_at(_noise: NoiseParams, _layering: Option<chico::LayeringKind>, _xz: Vec2) -> u8 {
	4
}

fn stub_kind_at(
	_noise: NoiseParams,
	pinned: Option<UrbanizationKind>,
	_xz: Vec2,
) -> UrbanizationKind {
	pinned.unwrap_or(UrbanizationKind::None)
}

#[test]
fn present_stream_follows_the_viewer_without_generate() -> anyhow::Result<()> {
	use bevy::prelude::Camera3d;

	let mut app = App::new();
	app.init_resource::<MobGenerateBullseye>();
	app.init_resource::<MobPresentBullseye>();
	app.init_resource::<LodGenerateKeepRegion<MobLodChan>>();
	app.init_resource::<LodPresentKeepRegion<MobLodChan>>();
	app.add_message::<lod::gen::LodGenerateRegion<MobLodChan>>()
		.add_message::<lod::presentation::LodPresentRegion<MobLodChan>>()
		.add_systems(bevy::prelude::Update, stream_mob_present);
	app.world_mut()
		.spawn((Camera3d::default(), Transform::from_xyz(50.0, 0.0, -20.0)));
	app.update();
	anyhow::ensure!(app.world().resource::<MobPresentBullseye>().enabled);
	anyhow::ensure!(
		app.world().resource::<LodPresentKeepRegion<MobLodChan>>().region.is_some(),
		"present keep follows the viewer"
	);
	anyhow::ensure!(
		app.world().resource::<LodGenerateKeepRegion<MobLodChan>>().region.is_none(),
		"present stream does not arm generate"
	);
	Ok(())
}

#[test]
fn generate_stream_arms_the_grid() -> anyhow::Result<()> {
	use bevy::prelude::Camera3d;

	let mut app = App::new();
	app.init_resource::<MobGenerateBullseye>();
	app.init_resource::<LodGenerateKeepRegion<MobLodChan>>();
	app.init_resource::<crate::stream::MobGenerateStreamCell>();
	app.add_message::<lod::gen::LodGenerateRegion<MobLodChan>>()
		.add_systems(bevy::prelude::Update, stream_mob_generate);
	app.world_mut()
		.spawn((Camera3d::default(), Transform::from_xyz(50.0, 0.0, -20.0)));
	app.update();
	anyhow::ensure!(app.world().resource::<MobGenerateBullseye>().enabled);
	anyhow::ensure!(
		app.world().resource::<LodGenerateKeepRegion<MobLodChan>>().region.is_some(),
		"generate keep follows the viewer"
	);
	Ok(())
}

#[test]
fn insert_and_remove_one_cell() -> anyhow::Result<()> {
	let mut index = MobIndex::ready();
	let extent = MobCellExtent::from_cell_index(0, 0);
	let id = index.insert_cell(MobCell { extent, groups: Vec::new() });
	anyhow::ensure!(index.get(id).is_some(), "insert stores the cell");
	anyhow::ensure!(index.remove_cell(id).is_some(), "remove returns the cell");
	anyhow::ensure!(index.is_empty(), "remove drops the cell");
	Ok(())
}

struct SilentMode;

impl layer_stack::GenerationMode for SilentMode {}

struct OtherMode;

impl layer_stack::GenerationMode for OtherMode {}

struct Silent;

struct SilentCell;

impl TerrainCell for SilentCell {
	type Mesh = ();
	fn bounds(&self) -> Aabb3d {
		Aabb3d::new(Vec3::ZERO, Vec3::ZERO)
	}
	fn mesh_builder(&self) {}
	fn chunk_pose(&self) -> Transform {
		Transform::IDENTITY
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
		_prepare: &mut SystemParamItem<'_, '_, Self::Prepare>,
		_bounds: Aabb3d,
		_lod_ref: &LodRef,
	) {
	}
	fn height_at(_read: &SystemParamItem<'_, '_, Self::Read>, _xz: Vec2) -> Option<f32> {
		None
	}
	fn fallback_height_at(_read: &SystemParamItem<'_, '_, Self::Read>, _xz: Vec2) -> f32 {
		0.0
	}
	fn overlay_cell<'a>(
		_read: &'a SystemParamItem<'_, '_, Self::Read>,
		_bounds: Aabb3d,
		_target_size: f32,
		_overlay_size_tolerance: Option<f32>,
	) -> Option<&'a dyn TerrainCell<Mesh = ()>> {
		None
	}
	fn snapshot(_read: &SystemParamItem<'_, '_, Self::Read>, _region: Aabb3d) -> Self::Snapshot {
		SilentSnapshot
	}
	fn require_generation(_app: &App) {}
}

impl PlantHosts for Silent {
	fn plant_hosts(
		_read: &SystemParamItem<'_, '_, Self::Read>,
		_region: Aabb3d,
	) -> Vec<MobPlantHost> {
		Vec::new()
	}
}

impl UrbanSelection for Silent {
	fn selection(
		_read: &SystemParamItem<'_, '_, Self::Read>,
	) -> (NoiseParams, Option<UrbanizationKind>) {
		(NoiseParams::default(), None)
	}
	fn kind_at(
		_noise: NoiseParams,
		pinned: Option<UrbanizationKind>,
		_xz: Vec2,
	) -> UrbanizationKind {
		pinned.unwrap_or(UrbanizationKind::None)
	}
}

impl DiscoverablePlaces for Silent {
	type Places = ();
	fn places(_read: &SystemParamItem<'_, '_, Self::Places>) -> Vec<MobPlantHost> {
		Vec::new()
	}
}

impl SelectUrbanization for Silent {
	type Select = ();
	fn ensure_selected(_select: &mut SystemParamItem<'_, '_, Self::Select>, _region: Aabb3d) {}
}

struct SilentVeg;

impl VegetationModel for SilentVeg {
	type Ground = Silent;
	fn require_generation(app: &App) {
		app.require_layer::<LayerGenerationCore<Vegetation<Self>>, Vegetation<Self>>();
	}
}

impl VegetationGeneration for SilentVeg {
	const LABEL: &'static str = "stub";
	type Config = ();
	fn install_generation(_app: &mut App) {}
	fn apply_generation(_world: &mut World, _config: &()) {}
	fn clear_generation(_world: &mut World) {}
	fn install_presentation(_app: &mut App) {}
}

impl ForestSelection for SilentVeg {
	type Read = ();
	fn pick(
		_read: &SystemParamItem<'_, '_, Self::Read>,
	) -> (NoiseParams, Option<chico::LayeringKind>) {
		(NoiseParams::default(), None)
	}
	fn layers_at(_noise: NoiseParams, _layering: Option<chico::LayeringKind>, _xz: Vec2) -> u8 {
		0
	}
}

impl Scheme<Vegetation<SilentVeg>> for SilentMode {
	fn install(_app: &mut App, _config: &()) {}
}

impl Scheme<Vegetation<SilentVeg>> for OtherMode {
	fn install(_app: &mut App, _config: &()) {}
}

impl Scheme<Mobs<Barking<Vegetation<SilentVeg>>>> for SilentMode {
	fn install(_app: &mut App, _config: &BarkingConfig) {}
}

impl Scheme<Mobs<Barking<Vegetation<SilentVeg>>>> for OtherMode {
	fn install(_app: &mut App, _config: &BarkingConfig) {}
}

fn plugin_app() -> App {
	use bevy::prelude::{AssetPlugin, MinimalPlugins};
	use bevy::state::app::StatesPlugin;
	use layer_stack::GenerationModePlugin;

	let mut app = App::new();
	app.add_plugins((
		MinimalPlugins,
		AssetPlugin::default(),
		StatesPlugin,
		GenerationModePlugin::<SilentMode>::initial(),
		GenerationModePlugin::<OtherMode>::default(),
		Generate::<SilentMode, Vegetation<SilentVeg>>::default(),
		Generate::<OtherMode, Vegetation<SilentVeg>>::default(),
		Generate::<SilentMode, Mobs<Barking<Vegetation<SilentVeg>>>>::new(
			BarkingConfig::world_defaults(),
		),
		Generate::<OtherMode, Mobs<Barking<Vegetation<SilentVeg>>>>::new(
			BarkingConfig::world_defaults(),
		),
	));
	app
}

#[test]
fn generation_without_vegetation_names_the_missing_plugin() {
	let result = std::panic::catch_unwind(|| {
		Generate::<SilentMode, Mobs<Barking<Vegetation<SilentVeg>>>>::default()
			.finish(&mut App::new());
	});
	let message = match result {
		Ok(()) => "plugin finish returned".to_string(),
		Err(payload) => payload
			.downcast_ref::<String>()
			.cloned()
			.or_else(|| payload.downcast_ref::<&str>().map(|text| (*text).to_string()))
			.unwrap_or_else(|| "non-string panic".to_string()),
	};
	assert!(
		message.contains("LayerGenerationCore"),
		"finish names the missing vegetation core, got {message}"
	);
}

#[test]
fn two_modes_install_the_core_once() -> anyhow::Result<()> {
	let mut app = plugin_app();
	app.finish();
	anyhow::ensure!(
		app.is_plugin_added::<LayerGenerationCore<Mobs<Barking<Vegetation<SilentVeg>>>>>(),
		"core is installed once"
	);
	Ok(())
}

#[test]
fn leaving_a_mode_clears_the_index() -> anyhow::Result<()> {
	use bevy::prelude::NextState;
	use layer_stack::ActiveGenerationMode;

	let mut app = plugin_app();
	app.finish();
	let extent = MobCellExtent::from_cell_index(0, 0);
	app.world_mut()
		.resource_mut::<MobIndex>()
		.insert_cell(MobCell { extent, groups: Vec::new() });
	app.world_mut()
		.resource_mut::<NextState<ActiveGenerationMode>>()
		.set(ActiveGenerationMode::of::<OtherMode>());
	app.update();
	anyhow::ensure!(app.world().resource::<MobIndex>().is_empty(), "OnExit clears every mob cell");
	Ok(())
}

#[test]
fn mob_cell_writes_insert_announces() -> anyhow::Result<()> {
	use bevy::ecs::message::Messages;
	use bevy::ecs::system::RunSystemOnce;
	use bevy::prelude::MessageReader;
	use lod::gen::LodGenerated;

	use crate::MobCellWrites;

	let mut world = World::new();
	world.init_resource::<Messages<LodGenerated<MobCell>>>();
	world.insert_resource(MobIndex::default());
	let extent = MobCellExtent::from_cell_index(0, 0);
	let id = world
		.run_system_once(move |mut cells: MobCellWrites| {
			cells.insert(MobCell { extent, groups: Vec::new() })
		})
		.map_err(|error| anyhow::anyhow!("{error:?}"))?;
	anyhow::ensure!(world.resource::<MobIndex>().get(id).is_some(), "insert stores the cell");
	let announced = world
		.run_system_once(|mut reader: MessageReader<LodGenerated<MobCell>>| {
			reader.read().map(|message| message.id).collect::<Vec<_>>()
		})
		.map_err(|error| anyhow::anyhow!("{error:?}"))?;
	anyhow::ensure!(announced == vec![id], "insert announces LodGenerated for the cell");
	Ok(())
}

#[test]
fn different_budgets_build_and_apply_on_enter() -> anyhow::Result<()> {
	use bevy::prelude::{AssetPlugin, MinimalPlugins, NextState};
	use bevy::state::app::StatesPlugin;
	use layer_stack::{ActiveGenerationMode, GenerationModePlugin};
	use lod::gen::LodGenerateBudget;

	let mut app = App::new();
	app.add_plugins((
		MinimalPlugins,
		AssetPlugin::default(),
		StatesPlugin,
		GenerationModePlugin::<SilentMode>::initial(),
		GenerationModePlugin::<OtherMode>::default(),
		Generate::<SilentMode, Vegetation<SilentVeg>>::default(),
		Generate::<OtherMode, Vegetation<SilentVeg>>::default(),
		Generate::<SilentMode, Mobs<Barking<Vegetation<SilentVeg>>>>::new(
			BarkingConfig::world_defaults(),
		),
		Generate::<OtherMode, Mobs<Barking<Vegetation<SilentVeg>>>>::new(BarkingConfig {
			generate_budget: 8,
		}),
	));
	app.finish();
	app.update();
	anyhow::ensure!(
		app.world().resource::<LodGenerateBudget<MobLodChan>>().ids_per_frame == 16,
		"initial budget"
	);
	app.world_mut()
		.resource_mut::<NextState<ActiveGenerationMode>>()
		.set(ActiveGenerationMode::of::<OtherMode>());
	app.update();
	anyhow::ensure!(
		app.world().resource::<LodGenerateBudget<MobLodChan>>().ids_per_frame == 8,
		"other mode budget"
	);
	Ok(())
}

#[test]
fn grid_stream_resumes_after_another_mode() -> anyhow::Result<()> {
	use bevy::ecs::message::Messages;
	use bevy::prelude::{AssetPlugin, Camera3d, MinimalPlugins, NextState};
	use bevy::state::app::StatesPlugin;
	use layer_stack::{ActiveGenerationMode, GenerationModePlugin};
	use lod::gen::{LodGenerateQueue, LodGenerateRegion};

	use crate::stream::install_mob_grid_stream;

	let mut app = App::new();
	app.add_plugins((
		MinimalPlugins,
		AssetPlugin::default(),
		StatesPlugin,
		GenerationModePlugin::<SilentMode>::initial(),
		GenerationModePlugin::<OtherMode>::default(),
	));
	app.init_resource::<MobGenerateBullseye>();
	app.init_resource::<LodGenerateKeepRegion<MobLodChan>>();
	app.init_resource::<LodGenerateQueue<MobCell>>();
	app.add_message::<LodGenerateRegion<MobLodChan>>();
	install_mob_grid_stream::<SilentMode>(&mut app);
	app.world_mut()
		.spawn((Camera3d::default(), Transform::from_xyz(50.0, 0.0, -20.0)));
	app.update();
	anyhow::ensure!(
		app.world().resource::<LodGenerateKeepRegion<MobLodChan>>().region.is_some(),
		"grid generate keep follows the viewer"
	);
	let leftover = MobCellExtent::from_cell_index(4, 1).id();
	app.world_mut().resource_mut::<LodGenerateQueue<MobCell>>().enqueue(leftover);
	app.world_mut()
		.resource_mut::<Messages<LodGenerateRegion<MobLodChan>>>()
		.drain()
		.for_each(drop);

	app.world_mut()
		.resource_mut::<NextState<ActiveGenerationMode>>()
		.set(ActiveGenerationMode::of::<OtherMode>());
	app.update();
	anyhow::ensure!(
		app.world().resource::<LodGenerateKeepRegion<MobLodChan>>().region.is_none(),
		"leaving the grid mode drops generate keep"
	);
	anyhow::ensure!(!app.world().resource::<MobGenerateBullseye>().enabled);
	anyhow::ensure!(
		app.world().resource::<LodGenerateQueue<MobCell>>().is_empty(),
		"leaving the grid mode drops leftover generate jobs"
	);

	app.world_mut()
		.resource_mut::<NextState<ActiveGenerationMode>>()
		.set(ActiveGenerationMode::of::<SilentMode>());
	app.update();
	anyhow::ensure!(
		app.world().resource::<LodGenerateKeepRegion<MobLodChan>>().region.is_some(),
		"re-entering the grid mode arms generate again"
	);
	let regions = app
		.world_mut()
		.resource_mut::<Messages<LodGenerateRegion<MobLodChan>>>()
		.drain()
		.count();
	anyhow::ensure!(
		regions > 0,
		"re-entering the grid mode writes a generate region without crossing a cell"
	);
	Ok(())
}

#[derive(bevy::prelude::Resource, Default)]
struct StubHostStore {
	hosts: Vec<MobPlantHost>,
	selected: bool,
}

struct StubHosts;

impl TerrainModel for StubHosts {
	type Base = Self;
	type Cell = SilentCell;
	type Read = Res<'static, StubHostStore>;
	type Snapshot = SilentSnapshot;
	type Prepare = ();

	fn prepare(
		_prepare: &mut SystemParamItem<'_, '_, Self::Prepare>,
		_bounds: Aabb3d,
		_lod_ref: &LodRef,
	) {
	}
	fn height_at(_read: &SystemParamItem<'_, '_, Self::Read>, _xz: Vec2) -> Option<f32> {
		None
	}
	fn fallback_height_at(_read: &SystemParamItem<'_, '_, Self::Read>, _xz: Vec2) -> f32 {
		0.0
	}
	fn overlay_cell<'a>(
		_read: &'a SystemParamItem<'_, '_, Self::Read>,
		_bounds: Aabb3d,
		_target_size: f32,
		_overlay_size_tolerance: Option<f32>,
	) -> Option<&'a dyn TerrainCell<Mesh = ()>> {
		None
	}
	fn snapshot(_read: &SystemParamItem<'_, '_, Self::Read>, _region: Aabb3d) -> Self::Snapshot {
		SilentSnapshot
	}
	fn require_generation(_app: &App) {}
}

impl PlantHosts for StubHosts {
	fn plant_hosts(
		read: &SystemParamItem<'_, '_, Self::Read>,
		_region: Aabb3d,
	) -> Vec<MobPlantHost> {
		read.hosts.clone()
	}
}

impl UrbanSelection for StubHosts {
	fn selection(
		_read: &SystemParamItem<'_, '_, Self::Read>,
	) -> (NoiseParams, Option<UrbanizationKind>) {
		(NoiseParams::default(), None)
	}
	fn kind_at(
		_noise: NoiseParams,
		pinned: Option<UrbanizationKind>,
		_xz: Vec2,
	) -> UrbanizationKind {
		pinned.unwrap_or(UrbanizationKind::None)
	}
}

impl DiscoverablePlaces for StubHosts {
	type Places = ();
	fn places(_read: &SystemParamItem<'_, '_, Self::Places>) -> Vec<MobPlantHost> {
		vec![MobPlantHost { xz: Vec2::new(3.0, 4.0), arrival_radius: 2.0 }]
	}
}

impl SelectUrbanization for StubHosts {
	type Select = bevy::prelude::ResMut<'static, StubHostStore>;
	fn ensure_selected(select: &mut SystemParamItem<'_, '_, Self::Select>, _region: Aabb3d) {
		select.selected = true;
	}
}

#[test]
fn stub_plant_hosts_come_from_the_trait() -> anyhow::Result<()> {
	let mut world = World::new();
	world.insert_resource(MobIndex::ready());
	world.insert_resource(StubHostStore {
		hosts: vec![MobPlantHost { xz: Vec2::new(1.0, 2.0), arrival_radius: 5.0 }],
		selected: false,
	});
	let region = Aabb3d::from_min_max(Vec3::new(-10.0, 0.0, -10.0), Vec3::new(10.0, 1.0, 10.0));
	world.insert_resource({
		let mut keep = LodGenerateKeepRegion::<MobLodChan>::default();
		keep.region = Some(region);
		keep
	});
	world
		.run_system_once(sync_mob_plant_hosts::<StubHosts>)
		.map_err(|error| anyhow::anyhow!("{error:?}"))?;
	let hosts = world.resource::<MobIndex>().plant_hosts.clone();
	anyhow::ensure!(hosts.len() == 2, "leaf host then discoverable place, got {}", hosts.len());
	anyhow::ensure!(hosts[0].xz == Vec2::new(1.0, 2.0));
	anyhow::ensure!(hosts[1].xz == Vec2::new(3.0, 4.0));
	anyhow::ensure!(world.resource::<StubHostStore>().selected, "ensure_selected ran");
	Ok(())
}
