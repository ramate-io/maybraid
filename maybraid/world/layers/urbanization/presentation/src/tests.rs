use bevy::app::{App, Plugin};
use bevy::ecs::system::{SystemParamItem, SystemState};
use bevy::math::bounding::Aabb3d;
use bevy::math::Vec2;
use bevy::prelude::{AssetPlugin, MinimalPlugins, NextState, World};
use bevy::state::app::StatesPlugin;
use layer_stack::{
	subscribe_mode, ActiveGenerationMode, GenerationMode, GenerationModePlugin, LayerGenerationCore,
	LayerPresentationCore, ModeSubscribers, ModeSubscription, Present, RequireLayer,
};
use lod::gen::{Id, Version};
use lod::lod_ref::LodRef;
use terrain_layer_model::{HeightField, OnTerrain, TerrainCell, TerrainGeneration, TerrainModel};
use urbanization_layer_model::{PadOps, Urbanization, UrbanizationGeneration, UrbanizationModel};

use crate::{PaddedCells, UrbanizationHosts};

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
		app.require_layer::<LayerGenerationCore<OnTerrain<Self>>, Self>();
	}
}

impl TerrainGeneration for SilentGround {
	const LABEL: &'static str = "silent-ground";
	type Config = ();
	fn install_generation(_app: &mut App) {}
	fn apply_generation(_world: &mut World, _config: &()) {}
	fn install_presentation(_app: &mut App) {}
}

#[derive(Clone, Copy, Default)]
struct SilentPads;

impl PadOps for SilentPads {
	fn modify_elevation(&self, height: f32, _x: f32, _z: f32) -> f32 {
		height
	}
}

struct SilentUrban;

impl UrbanizationModel for SilentUrban {
	type Ground = SilentGround;
	type Pads = SilentPads;
	type Surface = SilentCell;
	type Built = ();
	type Read = ();
	type Prepare = ();

	fn pads(_read: &SystemParamItem<'_, '_, Self::Read>, _region: Aabb3d) -> SilentPads {
		SilentPads
	}

	fn pads_at(_read: &SystemParamItem<'_, '_, Self::Read>, _xz: Vec2) -> SilentPads {
		SilentPads
	}

	fn surface<'a>(
		_read: &'a SystemParamItem<'_, '_, Self::Read>,
		_id: Id,
	) -> Option<&'a SilentCell> {
		None
	}

	fn surface_ids(_read: &SystemParamItem<'_, '_, Self::Read>, _region: Aabb3d) -> Vec<Id> {
		Vec::new()
	}

	fn overlay_surface<'a>(
		_read: &'a SystemParamItem<'_, '_, Self::Read>,
		_bounds: Aabb3d,
	) -> Option<&'a SilentCell> {
		None
	}

	fn built<'a>(_read: &'a SystemParamItem<'_, '_, Self::Read>, _region: Aabb3d) -> Vec<&'a ()> {
		Vec::new()
	}

	fn built_overlapping<'a>(
		_read: &'a SystemParamItem<'_, '_, Self::Read>,
		_region: Aabb3d,
	) -> Vec<(Id, Version, &'a ())> {
		Vec::new()
	}

	fn prepare(
		_prepare: &mut SystemParamItem<'_, '_, Self::Prepare>,
		_bounds: Aabb3d,
		_lod_ref: &LodRef,
	) {
	}

	fn require_generation(app: &App) {
		app.require_layer::<LayerGenerationCore<Urbanization<Self>>, Urbanization<Self>>();
	}
}

impl UrbanizationGeneration for SilentUrban {
	const LABEL: &'static str = "stub";
	type Config = ();
	fn install_generation(_app: &mut App) {}
	fn apply_generation(_world: &mut World, _config: &()) {}
	fn clear_generation(_world: &mut World) {}
	fn install_presentation(_app: &mut App) {}
}

#[test]
fn presentation_without_generation_names_the_missing_plugin() {
	let result = std::panic::catch_unwind(|| {
		Present::<TestMode, Urbanization<SilentUrban>>::default().finish(&mut App::new());
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
		message.contains("LayerGenerationCore"),
		"finish names the missing ground core, got {message}"
	);
}

struct OtherMode;

impl GenerationMode for OtherMode {}

type Stacked = Urbanization<SilentUrban>;

fn subscribed_app() -> App {
	let mut app = App::new();
	app.add_plugins((
		MinimalPlugins,
		StatesPlugin,
		GenerationModePlugin::<TestMode>::initial(),
		GenerationModePlugin::<OtherMode>::default(),
	));
	subscribe_mode::<(Stacked, UrbanizationHosts), TestMode>(&mut app);
	subscribe_mode::<(Stacked, PaddedCells), TestMode>(&mut app);
	app
}

#[test]
fn losing_subscription_is_inactive_and_returning_is_active() -> anyhow::Result<()> {
	let mut app = subscribed_app();
	app.update();
	{
		let mut state =
			SystemState::<ModeSubscription<(Stacked, UrbanizationHosts)>>::new(app.world_mut());
		anyhow::ensure!(
			state.get(app.world()).map_err(|error| anyhow::anyhow!("{error:?}"))?.active(),
			"hosts follow the subscribed mode"
		);
	}
	{
		let mut state =
			SystemState::<ModeSubscription<(Stacked, PaddedCells)>>::new(app.world_mut());
		anyhow::ensure!(
			state.get(app.world()).map_err(|error| anyhow::anyhow!("{error:?}"))?.active(),
			"padded follows the subscribed mode"
		);
	}

	app.world_mut()
		.resource_mut::<NextState<ActiveGenerationMode>>()
		.set(ActiveGenerationMode::of::<OtherMode>());
	app.update();
	{
		let mut state =
			SystemState::<ModeSubscription<(Stacked, UrbanizationHosts)>>::new(app.world_mut());
		anyhow::ensure!(
			!state.get(app.world()).map_err(|error| anyhow::anyhow!("{error:?}"))?.active(),
			"unsubscribed mode retires hosts"
		);
	}

	app.world_mut()
		.resource_mut::<NextState<ActiveGenerationMode>>()
		.set(ActiveGenerationMode::of::<TestMode>());
	app.update();
	{
		let mut state =
			SystemState::<ModeSubscription<(Stacked, UrbanizationHosts)>>::new(app.world_mut());
		anyhow::ensure!(
			state.get(app.world()).map_err(|error| anyhow::anyhow!("{error:?}"))?.active(),
			"return presents hosts again"
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
		Present::<TestMode, Urbanization<SilentUrban>>::default(),
		Present::<OtherMode, Urbanization<SilentUrban>>::default(),
	));
	anyhow::ensure!(
		app.is_plugin_added::<LayerPresentationCore<Urbanization<SilentUrban>>>(),
		"core is installed"
	);
	let hosts = app.world().resource::<ModeSubscribers<Stacked>>();
	anyhow::ensure!(hosts.contains::<TestMode>());
	anyhow::ensure!(hosts.contains::<OtherMode>());
	Ok(())
}
