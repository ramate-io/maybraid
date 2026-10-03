use bevy::app::{App, Plugin};
use bevy::ecs::system::{SystemParamItem, SystemState};
use bevy::math::bounding::Aabb3d;
use bevy::math::Vec2;
use bevy::prelude::{AssetPlugin, MinimalPlugins, NextState, World};
use bevy::state::app::StatesPlugin;
use layer_stack::{
	subscribe_mode, ActiveGenerationMode, GenerationMode, GenerationModePlugin, ModeSubscribers,
	ModeSubscription,
};
use lod::lod_ref::LodRef;
use terrain_layer_model::{HeightField, TerrainCell, TerrainModel};
use vegetation_layer_model::{Vegetation, VegetationGeneration, VegetationModel};

use crate::{
	VegetationPresent, VegetationPresentation, VegetationPresentationCore,
	VegetationPresentationPlugin,
};

struct TestMode;

impl GenerationMode for TestMode {}

struct SilentGround;

#[derive(Clone)]
struct SilentField;

impl HeightField for SilentField {
	fn height_at(&self, _xz: Vec2) -> Option<f32> {
		None
	}
	fn fallback_height_at(&self, _xz: Vec2) -> f32 {
		0.0
	}
}

struct SilentCell;

impl TerrainCell for SilentCell {
	type Mesh = ();
	fn bounds(&self) -> Aabb3d {
		Aabb3d::from_min_max(bevy::math::Vec3::ZERO, bevy::math::Vec3::ONE)
	}
	fn mesh_builder(&self) {}
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

impl TerrainModel for SilentGround {
	type Base = Self;
	type Cell = SilentCell;
	type Read = ();
	type Snapshot = SilentField;
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

	fn snapshot(_read: &SystemParamItem<'_, '_, Self::Read>, _region: Aabb3d) -> SilentField {
		SilentField
	}

	fn require_generation(app: &App) {
		layer_stack::RequireLayer::require_layer::<
			terrain_layer_model::BaseTerrainGenerationCore<Self>,
			Self,
		>(app);
	}
}

impl terrain_layer_model::TerrainGeneration for SilentGround {
	type Config = ();
	fn install_generation(_app: &mut App) {}
	fn apply_generation(_world: &mut World, _config: &()) {}
}

struct SilentVeg;

impl VegetationModel for SilentVeg {
	type Ground = SilentGround;

	fn require_generation(app: &App) {
		layer_stack::RequireLayer::require_layer::<
			vegetation_layer_model::VegetationGenerationCore<Self>,
			Vegetation<Self>,
		>(app);
	}
}

impl VegetationGeneration for SilentVeg {
	type Config = ();
	fn install_generation(_app: &mut App) {}
	fn apply_generation(_world: &mut World, _config: &()) {}
	fn clear_generation(_world: &mut World) {}
}

impl VegetationPresentation for SilentVeg {
	fn install_groves(_app: &mut App) {}
	fn install_bump_outs(_app: &mut App) {}
	fn install_materials(_app: &mut App) {}
}

#[test]
fn presentation_without_generation_names_the_missing_plugin() {
	let result = std::panic::catch_unwind(|| {
		VegetationPresentationPlugin::<TestMode, SilentVeg>::default().finish(&mut App::new());
	});
	let message = match result {
		Ok(()) => "plugin finish returned".to_string(),
		Err(payload) => payload
			.downcast_ref::<String>()
			.cloned()
			.or_else(|| payload.downcast_ref::<&str>().map(|s| (*s).to_string()))
			.unwrap_or_else(|| "non-string panic".to_string()),
	};
	assert!(
		message.contains("BaseTerrainGenerationCore"),
		"finish names the missing ground core, got {message}"
	);
}

struct OtherMode;

impl GenerationMode for OtherMode {}

type Stacked = Vegetation<SilentVeg>;

fn subscribed_app() -> App {
	let mut app = App::new();
	app.add_plugins((
		MinimalPlugins,
		StatesPlugin,
		GenerationModePlugin::<TestMode>::initial(),
		GenerationModePlugin::<OtherMode>::default(),
	));
	subscribe_mode::<(Stacked, VegetationPresent), TestMode>(&mut app);
	app
}

#[test]
fn losing_subscription_is_inactive_and_returning_is_active() -> anyhow::Result<()> {
	let mut app = subscribed_app();
	app.update();
	{
		let mut state =
			SystemState::<ModeSubscription<(Stacked, VegetationPresent)>>::new(app.world_mut());
		anyhow::ensure!(
			state.get(app.world()).map_err(|error| anyhow::anyhow!("{error:?}"))?.active(),
			"subscribed mode presents"
		);
	}

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

	app.world_mut()
		.resource_mut::<NextState<ActiveGenerationMode>>()
		.set(ActiveGenerationMode::of::<TestMode>());
	app.update();
	{
		let mut state =
			SystemState::<ModeSubscription<(Stacked, VegetationPresent)>>::new(app.world_mut());
		anyhow::ensure!(
			state.get(app.world()).map_err(|error| anyhow::anyhow!("{error:?}"))?.active(),
			"return presents again"
		);
	}
	Ok(())
}

#[test]
fn two_modes_install_the_core_once() -> anyhow::Result<()> {
	let mut app = App::new();
	app.add_plugins((MinimalPlugins, AssetPlugin::default(), StatesPlugin));
	app.add_plugins((
		GenerationModePlugin::<TestMode>::initial(),
		GenerationModePlugin::<OtherMode>::default(),
		VegetationPresentationPlugin::<TestMode, SilentVeg>::default(),
		VegetationPresentationPlugin::<OtherMode, SilentVeg>::default(),
	));
	anyhow::ensure!(
		app.is_plugin_added::<VegetationPresentationCore<SilentVeg>>(),
		"core is installed"
	);
	let subscribers = app.world().resource::<ModeSubscribers<(Stacked, VegetationPresent)>>();
	anyhow::ensure!(subscribers.contains::<TestMode>());
	anyhow::ensure!(subscribers.contains::<OtherMode>());
	Ok(())
}
