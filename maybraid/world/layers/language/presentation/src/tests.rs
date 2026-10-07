use bevy::ecs::system::SystemParamItem;
use bevy::math::bounding::Aabb3d;
use bevy::math::Vec2;
use bevy::prelude::{App, MinimalPlugins, NextState, World};
use bevy::state::app::StatesPlugin;
use language_layer_model::{Language, LanguageGeneration, LanguageModel};
use layer_stack::{
	ActiveGenerationMode, Generate, GenerationMode, GenerationModePlugin, LayerGenerationCore,
	ModeSubscribers, Present, RequireLayer, Scheme,
};
use lod::lod_ref::LodRef;
use lod::LodPresentGate;
use terrain_layer_model::{HeightField, OnTerrain, TerrainCell, TerrainGeneration, TerrainModel};
use vegetation_layer_model::{Vegetation, VegetationGeneration, VegetationModel};

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

struct SilentVeg;

impl VegetationModel for SilentVeg {
	type Ground = SilentGround;

	fn require_generation(app: &App) {
		app.require_layer::<LayerGenerationCore<Vegetation<Self>>, Vegetation<Self>>();
	}
}

impl VegetationGeneration for SilentVeg {
	const LABEL: &'static str = "silent-veg";
	type Config = ();
	fn install_generation(_app: &mut App) {}
	fn apply_generation(_world: &mut World, _config: &()) {}
	fn clear_generation(_world: &mut World) {}
	fn install_presentation(_app: &mut App) {}
}

struct SilentLanguage;

impl LanguageModel for SilentLanguage {
	type World = Vegetation<SilentVeg>;
	type Cell = ();

	fn require_generation(app: &App) {
		app.require_layer::<LayerGenerationCore<Language<Self>>, Language<Self>>();
	}
}

impl LanguageGeneration for SilentLanguage {
	const LABEL: &'static str = "silent-language";
	type Config = ();
	fn install_generation(_app: &mut App) {}
	fn apply_generation(_world: &mut World, _config: &()) {}
	fn clear_generation(_world: &mut World) {}
	fn install_presentation(_app: &mut App) {}
}

impl Scheme<Language<SilentLanguage>> for TestMode {
	fn install(_app: &mut App, _config: &()) {}
}

#[test]
fn presentation_follows_the_subscribed_mode() -> anyhow::Result<()> {
	let mut app = App::new();
	app.add_plugins((
		MinimalPlugins,
		StatesPlugin,
		GenerationModePlugin::<TestMode>::initial(),
		GenerationModePlugin::<OtherMode>::default(),
		LayerGenerationCore::<OnTerrain<SilentGround>>::default(),
		LayerGenerationCore::<Vegetation<SilentVeg>>::default(),
		Generate::<TestMode, Language<SilentLanguage>>::default(),
		Present::<TestMode, Language<SilentLanguage>>::default(),
	));
	app.finish();
	app.update();

	let subscribers = app.world().resource::<ModeSubscribers<Language<SilentLanguage>>>();
	anyhow::ensure!(subscribers.contains::<TestMode>(), "test mode is subscribed");
	anyhow::ensure!(!subscribers.contains::<OtherMode>(), "other mode is not subscribed");
	anyhow::ensure!(
		app.world().resource::<LodPresentGate<Language<SilentLanguage>>>().open,
		"subscribed mode opens the channel"
	);

	app.world_mut()
		.resource_mut::<NextState<ActiveGenerationMode>>()
		.set(ActiveGenerationMode::of::<OtherMode>());
	app.update();
	anyhow::ensure!(
		!app.world().resource::<LodPresentGate<Language<SilentLanguage>>>().open,
		"unsubscribed mode closes the channel"
	);
	Ok(())
}
