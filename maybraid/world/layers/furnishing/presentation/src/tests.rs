use bevy::ecs::system::SystemParamItem;
use bevy::math::bounding::Aabb3d;
use bevy::math::Vec2;
use bevy::prelude::{App, MinimalPlugins, NextState, World};
use bevy::state::app::StatesPlugin;
use furnishing_layer_model::{
	Furnishing, FurnishingGeneration, FurnishingGenerationCore, FurnishingGenerationPlugin,
	FurnishingModel, FurnishingScheme,
};
use layer_stack::{
	ActiveGenerationMode, GenerationMode, GenerationModePlugin, ModeSubscribers, RequireLayer,
};
use lod::lod_ref::LodRef;
use lod::LodPresentGate;
use terrain_layer_model::{
	BaseTerrainGenerationCore, HeightField, TerrainCell, TerrainGeneration, TerrainModel,
};

use crate::{FurnishingPresent, FurnishingPresentation, FurnishingPresentationPlugin};

struct TestMode;
struct OtherMode;

impl GenerationMode for TestMode {}
impl GenerationMode for OtherMode {}

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

struct SilentGround;

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
		app.require_layer::<BaseTerrainGenerationCore<Self>, Self>();
	}
}

impl TerrainGeneration for SilentGround {
	type Config = ();
	fn install_generation(_app: &mut App) {}
	fn apply_generation(_world: &mut World, _config: &()) {}
}

struct SilentFurnishing;

impl FurnishingModel for SilentFurnishing {
	type Ground = SilentGround;
	type Cell = ();

	fn require_generation(app: &App) {
		app.require_layer::<FurnishingGenerationCore<Self>, Furnishing<Self>>();
	}
}

impl FurnishingGeneration for SilentFurnishing {
	type Config = ();
	fn install_generation(_app: &mut App) {}
	fn apply_generation(_world: &mut World, _config: &()) {}
	fn clear_generation(_world: &mut World) {}
}

struct SilentChannel;

impl FurnishingScheme<SilentFurnishing> for TestMode {
	fn install(_app: &mut App, _config: &()) {}
}

impl FurnishingPresentation for SilentFurnishing {
	type Channel = SilentChannel;

	fn install_presentation(_app: &mut App) {}
}

#[test]
fn presentation_follows_the_subscribed_mode() -> anyhow::Result<()> {
	let mut app = App::new();
	app.add_plugins((
		MinimalPlugins,
		StatesPlugin,
		GenerationModePlugin::<TestMode>::initial(),
		GenerationModePlugin::<OtherMode>::default(),
		BaseTerrainGenerationCore::<SilentGround>::default(),
		FurnishingGenerationPlugin::<TestMode, SilentFurnishing>::default(),
		FurnishingPresentationPlugin::<TestMode, SilentFurnishing>::default(),
	));
	app.finish();
	app.update();

	let subscribers = app
		.world()
		.resource::<ModeSubscribers<(Furnishing<SilentFurnishing>, FurnishingPresent)>>();
	anyhow::ensure!(subscribers.contains::<TestMode>(), "test mode is subscribed");
	anyhow::ensure!(!subscribers.contains::<OtherMode>(), "other mode is not subscribed");
	anyhow::ensure!(
		app.world().resource::<LodPresentGate<SilentChannel>>().open,
		"subscribed mode opens the channel"
	);

	app.world_mut()
		.resource_mut::<NextState<ActiveGenerationMode>>()
		.set(ActiveGenerationMode::of::<OtherMode>());
	app.update();
	anyhow::ensure!(
		!app.world().resource::<LodPresentGate<SilentChannel>>().open,
		"unsubscribed mode closes the channel"
	);
	Ok(())
}
