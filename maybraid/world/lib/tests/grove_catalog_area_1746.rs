//! World-route GPU catalogs at Area 1746, through `WorldLayersPlugin`.
//!
//! This is the Discovery world `GamePlugin` / `WorldPlugin::game` installs
//! (`maybraid` binary, Tracy, `profiling/grove-lifecycle.py`). It is not the
//! tree-field playground.

use std::time::Duration;

use bevy::ecs::entity_disabling::Disabled;
use bevy::ecs::query::Allow;
use bevy::prelude::*;
use bevy::scene::WorldSceneExt;
use bevy::shape::Aabb3d;
use chico::{
	ChicoGroveHost, DEFAULT_FOREST_GROVE_TILE_XZ, ForestExtent, ForestGroveKind, ForestGroveRecipe,
	ForestLayer, GroveGpuVisuals,
};
use durham::Durham;
use lod::hcsg::SettlePresentation;
use lod::lod_ref::LodRef;
use lod::scene::LodScene;
use lod::{LodSceneLevel, LodViewer};
use maybraid_world::{TerrainStreaming, WorldLayersPlugin, WorldMaterialRefPlugin};
use vegetation_bush_field::{BushCatalogOp, BushCatalogOps, BushGroveCatalog, BushPatch};
use vegetation_components::{FoliageNode, StickNode};
use vegetation_grass_field::{GrassCatalogOp, GrassCatalogOps, GrassGroveCatalog, GrassPatch};
use vegetation_groves::{FlatTerrainSample, GroveExtent};
use vegetation_tree_field::{TreeCatalogOp, TreeCatalogOps, TreeGrove, TreeGroveCatalog};
use world_player::Player as WorldPlayer;

const AREA_1746: Vec3 = Vec3::new(1746.8, 20.0, 1037.9);

struct ExperimentEnv {
	mode: Option<String>,
	shadows: Option<String>,
}

impl ExperimentEnv {
	fn set_gpu() -> Self {
		let guard = Self {
			mode: std::env::var("MAYBRAID_EXPERIMENT_TREE_FIELD").ok(),
			shadows: std::env::var("MAYBRAID_SHADOWS").ok(),
		};
		unsafe {
			std::env::set_var("MAYBRAID_EXPERIMENT_TREE_FIELD", "gpu");
			std::env::set_var("MAYBRAID_SHADOWS", "off");
		}
		guard
	}
}

impl Drop for ExperimentEnv {
	fn drop(&mut self) {
		unsafe {
			match &self.mode {
				Some(value) => std::env::set_var("MAYBRAID_EXPERIMENT_TREE_FIELD", value),
				None => std::env::remove_var("MAYBRAID_EXPERIMENT_TREE_FIELD"),
			}
			match &self.shadows {
				Some(value) => std::env::set_var("MAYBRAID_SHADOWS", value),
				None => std::env::remove_var("MAYBRAID_SHADOWS"),
			}
		}
		GroveGpuVisuals::admit(false);
	}
}

fn world_app() -> App {
	let mut app = App::new();
	app.insert_resource(maybraid_virtual_pad_controller::PadHidPolicy::External);
	app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(Duration::from_millis(100)));
	app.set_error_handler(bevy::ecs::error::warn);
	app.add_plugins((
		MinimalPlugins,
		AssetPlugin::default(),
		bevy::mesh::MeshPlugin,
		bevy::state::app::StatesPlugin,
		bevy::scene::ScenePlugin,
		bevy::world_serialization::WorldSerializationPlugin,
		bevy::transform::TransformPlugin,
		bevy::input::InputPlugin,
		bevy::gizmos::GizmoPlugin,
		bevy::window::WindowPlugin { primary_window: None, ..default() },
	))
	.init_asset::<StandardMaterial>()
	.init_asset::<maybraid_audio::AudioClip>()
	.add_plugins((
		world_materials::WorldMaterialsShadersPlugin,
		WorldMaterialRefPlugin,
		player_camera::PlayerCameraPlugin,
		WorldLayersPlugin,
	));
	app.finish();
	app.cleanup();
	let at = Transform::from_translation(AREA_1746);
	app.world_mut().spawn((
		WorldPlayer,
		Camera3d::default(),
		LodViewer,
		at,
		GlobalTransform::default(),
	));
	app.insert_resource(TerrainStreaming::<Durham>::new(true));
	app
}

fn presenter_names(app: &mut App) -> Vec<String> {
	app.world_mut()
		.query::<&Name>()
		.iter(app.world())
		.map(|name| name.as_str().to_string())
		.filter(|name| {
			matches!(
				name.as_str(),
				"TreeFieldWood"
					| "TreeFieldFoliage"
					| "BushFieldWood"
					| "BushFieldFoliage"
					| "GrassFieldPresenter"
			)
		})
		.collect()
}

fn live_counts(app: &App) -> (usize, usize, usize) {
	(
		app.world().resource::<TreeGroveCatalog>().live_grove_count(),
		app.world().resource::<BushGroveCatalog>().live_grove_count(),
		app.world().resource::<GrassGroveCatalog>().live_grove_count(),
	)
}

fn admit_counts(app: &App) -> (usize, usize, usize) {
	let trees = app
		.world()
		.resource::<TreeCatalogOps>()
		.ops
		.iter()
		.filter(|op| matches!(op, TreeCatalogOp::Admit { .. }))
		.count();
	let bushes = app
		.world()
		.resource::<BushCatalogOps>()
		.ops
		.iter()
		.filter(|op| matches!(op, BushCatalogOp::Admit { .. }))
		.count();
	let grass = app
		.world()
		.resource::<GrassCatalogOps>()
		.ops
		.iter()
		.filter(|op| matches!(op, GrassCatalogOp::Admit { .. }))
		.count();
	(trees, bushes, grass)
}

fn nested_gpu_hosts(app: &mut App) -> usize {
	app.world_mut()
		.query_filtered::<(), (With<ChicoGroveHost>, Allow<Disabled>)>()
		.iter(app.world())
		.count()
}

fn kit_nodes(app: &mut App) -> (usize, usize) {
	let sticks = app
		.world_mut()
		.query_filtered::<(), (With<StickNode>, Allow<Disabled>)>()
		.iter(app.world())
		.count();
	let foliage = app
		.world_mut()
		.query_filtered::<(), (With<FoliageNode>, Allow<Disabled>)>()
		.iter(app.world())
		.count();
	(sticks, foliage)
}

fn area_1746_tile() -> GroveExtent {
	let s = DEFAULT_FOREST_GROVE_TILE_XZ;
	let c = Vec3::new(AREA_1746.x, 0.0, AREA_1746.z);
	ForestExtent::grove_tiles_overlapping(Aabb3d::from_min_max(
		c - Vec3::new(1.0, 0.0, 1.0),
		c + Vec3::new(1.0, 1.0, 1.0),
	))
	.into_iter()
	.next()
	.unwrap_or_else(|| {
		GroveExtent::new(c - Vec3::new(s * 0.5, 0.0, s * 0.5), c + Vec3::new(s * 0.5, 1.0, s * 0.5))
	})
}

fn spawn_stacked_high(app: &mut App) {
	let tile = area_1746_tile();
	let world = FlatTerrainSample::default();
	let stacked = [
		(ForestGroveKind::TemperateLowerMassives, ForestLayer::LowerCanopy),
		(ForestGroveKind::LowBush, ForestLayer::Understory),
		(ForestGroveKind::TallGrass, ForestLayer::Tufts),
	];
	let xf = Transform::from_translation(AREA_1746);
	let bounds = Aabb3d::from_min_max(Vec3::ZERO, Vec3::ONE);
	let lod_ref = LodRef {
		entity: Entity::PLACEHOLDER,
		previous_transform: &xf,
		current_transform: &xf,
		bounds: &bounds,
	};
	for (kind, layer) in stacked {
		let host = ChicoGroveHost::new(ForestGroveRecipe::uniform(kind, tile).grow(&world), layer);
		assert!(host.gpu_visuals().replaces_kits(), "{kind:?} at Area 1746");
		app.world_mut()
			.spawn_scene(host.scene_with_level(&lod_ref, LodSceneLevel::High))
			.unwrap_or_else(|err| panic!("{kind:?} High realize: {err}"));
	}
}

#[test]
fn gpu_env_world_layers_at_area_1746_admits_tree_bush_and_grass() -> anyhow::Result<()> {
	let _env = ExperimentEnv::set_gpu();
	let mut app = world_app();

	anyhow::ensure!(GroveGpuVisuals::admitted(), "gpu env must admit catalogs at plugin build");
	anyhow::ensure!(app.world().get_resource::<TreeGroveCatalog>().is_some());
	anyhow::ensure!(app.world().get_resource::<BushGroveCatalog>().is_some());
	anyhow::ensure!(app.world().get_resource::<GrassGroveCatalog>().is_some());

	app.update();
	let names = presenter_names(&mut app);
	for expected in [
		"TreeFieldWood",
		"TreeFieldFoliage",
		"BushFieldWood",
		"BushFieldFoliage",
		"GrassFieldPresenter",
	] {
		anyhow::ensure!(names.iter().any(|name| name == expected), "missing presenter {expected}");
	}

	let viewer = app
		.world_mut()
		.query_filtered::<&Transform, With<LodViewer>>()
		.single(app.world())
		.expect("LodViewer");
	anyhow::ensure!((viewer.translation.x - AREA_1746.x).abs() < 0.01);
	anyhow::ensure!((viewer.translation.z - AREA_1746.z).abs() < 0.01);

	let settle = SettlePresentation::layer_integration(Duration::from_secs(30));
	let streamed = settle
		.wait_until(&mut app, 48, |app| {
			let (trees, bushes, grass) = live_counts(app);
			nested_gpu_hosts(app) > 0 && trees > 0 && bushes > 0 && grass > 0
		})
		.is_ok();
	if !streamed {
		spawn_stacked_high(&mut app);
		app.update();
	}

	let (trees, bushes, grass) = live_counts(&app);
	anyhow::ensure!(trees > 0, "tree catalog Admits at Area 1746, streamed={streamed}");
	anyhow::ensure!(bushes > 0, "bush catalog Admits at Area 1746, streamed={streamed}");
	anyhow::ensure!(grass > 0, "grass catalog Admits at Area 1746, streamed={streamed}");

	let (tree_admits, bush_admits, grass_admits) = admit_counts(&app);
	anyhow::ensure!(tree_admits > 0, "TreeCatalogOp::Admit");
	anyhow::ensure!(bush_admits > 0, "BushCatalogOp::Admit");
	anyhow::ensure!(grass_admits > 0, "GrassCatalogOp::Admit");

	anyhow::ensure!(app.world_mut().query::<&TreeGrove>().iter(app.world()).count() > 0);
	anyhow::ensure!(app.world_mut().query::<&BushPatch>().iter(app.world()).count() > 0);
	anyhow::ensure!(app.world_mut().query::<&GrassPatch>().iter(app.world()).count() > 0);

	let (sticks, foliage) = kit_nodes(&mut app);
	anyhow::ensure!(sticks == 0, "GPU arm spawned StickNode kits: {sticks}");
	anyhow::ensure!(foliage == 0, "GPU arm spawned FoliageNode kits: {foliage}");

	eprintln!(
		"area-1746 gpu world: streamed={streamed} hosts={} trees={trees} bushes={bushes} grass={grass} admits={tree_admits}/{bush_admits}/{grass_admits}",
		nested_gpu_hosts(&mut app)
	);
	Ok(())
}
