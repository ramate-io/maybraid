use bevy::app::{App, Plugin};
use bevy::math::bounding::Aabb3d;
use bevy::math::{Vec2, Vec3};
use bevy::prelude::{Entity, GlobalTransform, Transform, Vec3Swizzles, World};
use chico::ForestIndex;
use durham::{
	BaseTerrainNoise, Durham, TerrainCellLayout, TerrainConfig, TerrainEntryStore, WorldBaseTerrain,
};
use lod::gen::{GenerationScheme, Id, LodGenerateKeepRegion, SpatialIndex};
use lod::lod_ref::LodRef;
use lod::presentation::LodPresentKeepRegion;
use barking::{GroupKind, MobGroup, MobPlantHost};
use procedural_common::NoiseParams;
use richmond::{DevelopmentConfig, DevelopmentEntryStore};
use urbanization_cells::{
	DevelopmentLeaf, SelectedUrbanization, UrbanDevelopmentKind, UrbanizationExtent,
	UrbanizationIndex, UrbanizationKind,
};
use terrain_layer_model::OnTerrain;
use urbanization_layer_model::{Urbanization, UrbanSetting};

use crate::index::{urban_leaf_arrival_radius, MobCell, MobCellExtent, MobIndex};
use crate::stream::{
	stream_mob_generate, stream_mob_present, sync_mob_models, sync_mob_plant_hosts,
	MobGenerateBullseye, MobLodChan, MobPresentBullseye,
};
use crate::{MobGenerationPlugin, MobLayerConfig, MobScheme};

type Urbanized = Urbanization<OnTerrain<Durham>>;

fn insert_urbanized_resources(world: &mut World) {
	world.insert_resource(TerrainEntryStore::default());
	world.insert_resource(TerrainCellLayout::default());
	world.insert_resource(WorldBaseTerrain(BaseTerrainNoise::from_config(&TerrainConfig::new(42))));
	world.insert_resource(DevelopmentEntryStore::default());
	world.insert_resource(UrbanizationIndex::default());
}

fn identity_lod_ref<'a>(transform: &'a Transform, bounds: &'a Aabb3d) -> LodRef<'a> {
	LodRef {
		entity: Entity::PLACEHOLDER,
		previous_transform: transform,
		current_transform: transform,
		bounds,
	}
}

#[test]
fn mob_cells_cover_a_four_hundred_metre_lattice() {
	let region =
		Aabb3d::from_min_max(Vec3::new(-199.0, 0.0, -199.0), Vec3::new(201.0, 1.0, 201.0));
	assert_eq!(MobCellExtent::cells_overlapping(region).len(), 4);
}

#[test]
fn origin_cell_is_always_populated_when_models_are_ready() -> anyhow::Result<()> {
	let mut index = MobIndex::ready();
	let extent = MobCellExtent::from_cell_index(0, 0);
	let transform = Transform::IDENTITY;
	let bounds = extent.aabb();
	let lod_ref = identity_lod_ref(&transform, &bounds);
	let (cell, _) = MobCell::build_with_id(&mut index, extent.id(), &lod_ref)
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
	let origin = Vec2::ZERO;
	let group = MobGroup::generate(GroupKind::Frontier, 11, origin, &index);
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
	app.add_systems(bevy::prelude::Update, sync_mob_models::<Urbanized>);
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
		leaves: vec![DevelopmentLeaf { bounds: leaf_bounds, kind: UrbanDevelopmentKind::LesHalles }],
	};
	let transform = Transform::IDENTITY;
	let bounds = extent.aabb();
	let lod_ref = identity_lod_ref(&transform, &bounds);
	SpatialIndex::<SelectedUrbanization>::insert(
		&mut *app.world_mut().resource_mut::<UrbanizationIndex>(),
		extent.id(),
		selected,
		bounds,
		&lod_ref,
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
		richmond::DiscoverablePlace::host(
			richmond::DiscoverablePlaceLabel::Storey,
			9.0,
			1.1,
		),
		GlobalTransform::from_translation(place_at),
	));

	app.add_systems(bevy::prelude::Update, sync_mob_plant_hosts::<Urbanized>);
	app.update();
	let world = app.world();

	let hosts = world.resource::<MobIndex>().plant_hosts.clone();
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
	assert_eq!(
		hosts[2],
		MobPlantHost { xz: setting_at.xz(), arrival_radius: 14.0 }
	);
	assert_eq!(
		hosts[3],
		MobPlantHost { xz: place_at.xz(), arrival_radius: 9.0 }
	);

	let urbanization = world.resource::<UrbanizationIndex>();
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
fn present_stream_follows_the_viewer_without_generate() -> anyhow::Result<()> {
	use bevy::prelude::{Camera3d, Transform};

	let mut app = App::new();
	app.init_resource::<MobGenerateBullseye>();
	app.init_resource::<MobPresentBullseye>();
	app.init_resource::<lod::gen::LodGenerateKeepRegion<MobLodChan>>();
	app.init_resource::<LodPresentKeepRegion<MobLodChan>>();
	app.add_message::<lod::gen::LodGenerateRegion<MobLodChan>>()
		.add_message::<lod::presentation::LodPresentRegion<MobLodChan>>()
		.add_systems(bevy::prelude::Update, stream_mob_present);
	app.world_mut().spawn((Camera3d::default(), Transform::from_xyz(50.0, 0.0, -20.0)));
	app.update();

	anyhow::ensure!(app.world().resource::<MobPresentBullseye>().enabled);
	anyhow::ensure!(
		app.world().resource::<LodPresentKeepRegion<MobLodChan>>().region.is_some(),
		"present keep follows the viewer"
	);
	anyhow::ensure!(
		app.world().resource::<lod::gen::LodGenerateKeepRegion<MobLodChan>>().region.is_none(),
		"present stream does not arm generate"
	);
	Ok(())
}

#[test]
fn generate_stream_arms_the_grid() -> anyhow::Result<()> {
	use bevy::prelude::{Camera3d, Transform};

	let mut app = App::new();
	app.init_resource::<MobGenerateBullseye>();
	app.init_resource::<lod::gen::LodGenerateKeepRegion<MobLodChan>>();
	app.init_resource::<crate::stream::MobGenerateStreamCell>();
	app.add_message::<lod::gen::LodGenerateRegion<MobLodChan>>()
		.add_systems(bevy::prelude::Update, stream_mob_generate);
	app.world_mut().spawn((Camera3d::default(), Transform::from_xyz(50.0, 0.0, -20.0)));
	app.update();

	anyhow::ensure!(app.world().resource::<MobGenerateBullseye>().enabled);
	anyhow::ensure!(
		app.world().resource::<lod::gen::LodGenerateKeepRegion<MobLodChan>>().region.is_some(),
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

#[test]
#[should_panic(expected = "VegetationGenerationCore")]
fn generation_without_vegetation_names_the_missing_plugin() {
	MobGenerationPlugin::<SilentMode, Silent>::default().finish(&mut App::new());
}

struct SilentMode;

impl layer_stack::GenerationMode for SilentMode {}

impl MobScheme<Silent> for SilentMode {
	fn install(_app: &mut App, _config: &MobLayerConfig) {}
}

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

impl terrain_layer_model::HeightField for SilentSnapshot {
	fn height_at(&self, _xz: Vec2) -> Option<f32> {
		None
	}

	fn fallback_height_at(&self, _xz: Vec2) -> f32 {
		0.0
	}
}

impl terrain_layer_model::TerrainModel for Silent {
	type Cell = SilentCell;
	type Read = ();
	type Snapshot = SilentSnapshot;
	type Prepare = ();

	fn prepare(
		_prepare: &mut bevy::ecs::system::SystemParamItem<'_, '_, Self::Prepare>,
		_bounds: Aabb3d,
		_lod_ref: &LodRef,
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

	fn cell_ids_overlapping(
		_read: &bevy::ecs::system::SystemParamItem<'_, '_, Self::Read>,
		_region: Aabb3d,
	) -> Vec<Id> {
		Vec::new()
	}

	fn cell<'a>(
		_read: &'a bevy::ecs::system::SystemParamItem<'_, '_, Self::Read>,
		_id: Id,
	) -> Option<&'a SilentCell> {
		None
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

impl urbanization_layer_model::UrbanModel for Silent {
	fn pads(
		_read: &bevy::ecs::system::SystemParamItem<'_, '_, Self::Read>,
		_region: Aabb3d,
	) -> richmond::PadComplex {
		richmond::PadComplex::new(procedural_common::Bounds2::new(
			Vec2::ZERO,
			Vec2::ZERO,
		))
	}

	fn urbanization_leaves<'a>(
		_read: &'a bevy::ecs::system::SystemParamItem<'_, '_, Self::Read>,
		_region: Aabb3d,
	) -> Vec<&'a DevelopmentLeaf> {
		Vec::new()
	}

	fn development_cells<'a>(
		_read: &'a bevy::ecs::system::SystemParamItem<'_, '_, Self::Read>,
		_region: Aabb3d,
	) -> Vec<&'a richmond::DevelopmentCell> {
		Vec::new()
	}

	fn built<'a>(
		_read: &'a bevy::ecs::system::SystemParamItem<'_, '_, Self::Read>,
		_region: Aabb3d,
	) -> Vec<&'a richmond::BuiltDevelopment> {
		Vec::new()
	}

	fn built_overlapping<'a>(
		_read: &'a bevy::ecs::system::SystemParamItem<'_, '_, Self::Read>,
		_region: Aabb3d,
	) -> Vec<(
		Id,
		lod::gen::Version,
		&'a richmond::BuiltDevelopment,
	)> {
		Vec::new()
	}

	fn development_cell<'a>(
		_read: &'a bevy::ecs::system::SystemParamItem<'_, '_, Self::Read>,
		_id: Id,
	) -> Option<&'a richmond::DevelopmentCell> {
		None
	}

	fn urbanization_selection(
		_read: &bevy::ecs::system::SystemParamItem<'_, '_, Self::Read>,
	) -> (NoiseParams, Option<UrbanizationKind>) {
		(NoiseParams::default(), None)
	}

	type Select = ();

	fn ensure_selected(
		_select: &mut bevy::ecs::system::SystemParamItem<'_, '_, Self::Select>,
		_region: Aabb3d,
	) {
	}
}

struct OtherMode;

impl layer_stack::GenerationMode for OtherMode {}

impl MobScheme<Silent> for OtherMode {
	fn install(_app: &mut App, _config: &MobLayerConfig) {}
}

fn plugin_app() -> App {
	use bevy::prelude::{AssetPlugin, MinimalPlugins};
	use bevy::state::app::StatesPlugin;
	use layer_stack::GenerationModePlugin;
	use terrain_layer_model::TerrainStreaming;
	use vegetation_layer_model::VegetationGenerationPlugin;

	let mut app = App::new();
	app.add_plugins((
		MinimalPlugins,
		AssetPlugin::default(),
		StatesPlugin,
		GenerationModePlugin::<SilentMode>::initial(),
		GenerationModePlugin::<OtherMode>::default(),
		VegetationGenerationPlugin::<SilentMode, Silent>::default(),
		VegetationGenerationPlugin::<OtherMode, Silent>::default(),
		MobGenerationPlugin::<SilentMode, Silent>::new(MobLayerConfig::world_defaults()),
		MobGenerationPlugin::<OtherMode, Silent>::new(MobLayerConfig::world_defaults()),
	));
	app.insert_resource(TerrainStreaming::<Silent>::new(false));
	app.init_resource::<ForestIndex>();
	app
}

#[test]
fn two_modes_install_the_core_once() -> anyhow::Result<()> {
	let mut app = plugin_app();
	app.finish();
	anyhow::ensure!(
		app.is_plugin_added::<crate::MobGenerationCore<Silent>>(),
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
	anyhow::ensure!(
		app.world().resource::<MobIndex>().is_empty(),
		"OnExit clears every mob cell"
	);
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
fn grid_stream_resumes_after_another_mode() -> anyhow::Result<()> {
	use bevy::ecs::message::Messages;
	use bevy::prelude::{AssetPlugin, Camera3d, MinimalPlugins, NextState, Transform};
	use bevy::state::app::StatesPlugin;
	use lod::gen::{LodGenerateQueue, LodGenerateRegion};
	use crate::stream::{install_mob_grid_stream, MobGenerateBullseye};
	use layer_stack::{ActiveGenerationMode, GenerationModePlugin};

	let mut app = App::new();
	app.add_plugins((
		MinimalPlugins,
		AssetPlugin::default(),
		StatesPlugin,
		GenerationModePlugin::<SilentMode>::initial(),
		GenerationModePlugin::<OtherMode>::default(),
	));
	app.init_resource::<MobGenerateBullseye>();
	app.init_resource::<lod::gen::LodGenerateKeepRegion<MobLodChan>>();
	app.init_resource::<LodGenerateQueue<MobCell>>();
	app.add_message::<LodGenerateRegion<MobLodChan>>();
	install_mob_grid_stream::<SilentMode>(&mut app);
	app.world_mut().spawn((Camera3d::default(), Transform::from_xyz(50.0, 0.0, -20.0)));
	app.update();
	anyhow::ensure!(
		app.world().resource::<lod::gen::LodGenerateKeepRegion<MobLodChan>>().region.is_some(),
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
		app.world().resource::<lod::gen::LodGenerateKeepRegion<MobLodChan>>().region.is_none(),
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
		app.world().resource::<lod::gen::LodGenerateKeepRegion<MobLodChan>>().region.is_some(),
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

#[test]
fn different_budgets_build_and_apply_on_enter() -> anyhow::Result<()> {
	use bevy::prelude::{AssetPlugin, MinimalPlugins, NextState};
	use bevy::state::app::StatesPlugin;
	use layer_stack::{ActiveGenerationMode, GenerationModePlugin};
	use lod::gen::LodGenerateBudget;
	use terrain_layer_model::TerrainStreaming;
	use vegetation_layer_model::VegetationGenerationPlugin;

	use crate::MobLodChan;

	let mut app = App::new();
	app.add_plugins((
		MinimalPlugins,
		AssetPlugin::default(),
		StatesPlugin,
		GenerationModePlugin::<SilentMode>::initial(),
		GenerationModePlugin::<OtherMode>::default(),
		VegetationGenerationPlugin::<SilentMode, Silent>::default(),
		VegetationGenerationPlugin::<OtherMode, Silent>::default(),
		MobGenerationPlugin::<SilentMode, Silent>::new(MobLayerConfig::world_defaults()),
		MobGenerationPlugin::<OtherMode, Silent>::new(MobLayerConfig { generate_budget: 8 }),
	));
	app.insert_resource(TerrainStreaming::<Silent>::new(false));
	app.init_resource::<ForestIndex>();
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
fn plugin_order_does_not_matter() -> anyhow::Result<()> {
	use bevy::prelude::{AssetPlugin, MinimalPlugins};
	use bevy::state::app::StatesPlugin;
	use layer_stack::GenerationModePlugin;
	use terrain_layer_model::TerrainStreaming;
	use vegetation_layer_model::VegetationGenerationPlugin;

	let mut app = App::new();
	app.add_plugins((
		MinimalPlugins,
		AssetPlugin::default(),
		StatesPlugin,
		MobGenerationPlugin::<OtherMode, Silent>::new(MobLayerConfig { generate_budget: 8 }),
		MobGenerationPlugin::<SilentMode, Silent>::new(MobLayerConfig::world_defaults()),
		VegetationGenerationPlugin::<SilentMode, Silent>::default(),
		VegetationGenerationPlugin::<OtherMode, Silent>::default(),
		GenerationModePlugin::<SilentMode>::initial(),
		GenerationModePlugin::<OtherMode>::default(),
	));
	app.insert_resource(TerrainStreaming::<Silent>::new(false));
	app.init_resource::<ForestIndex>();
	app.finish();
	app.update();
	anyhow::ensure!(
		app.is_plugin_added::<crate::MobGenerationCore<Silent>>(),
		"core is installed"
	);
	Ok(())
}
