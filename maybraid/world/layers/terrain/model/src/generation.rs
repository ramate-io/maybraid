//! Base of the stack: models that own a generation pipeline.

use std::marker::PhantomData;

use bevy::app::{App, Plugin};
use bevy::prelude::*;

use layer_stack::{ActiveGenerationMode, GenerationMode};
use crate::model::TerrainModel;

/// A model with its own generation stack (the bottom of the wiring diagram).
///
/// Wrappers such as `Urbanization<M>` are not `TerrainGeneration`; their
/// generation plugins are typed by the model they read instead.
pub trait TerrainGeneration: TerrainModel {
	type Config: Clone + Send + Sync + 'static;

	/// Register the model's stores, layout, and generate systems. No presentation.
	fn install_generation(app: &mut App);

	/// Apply this mode's config. A seed change rebuilds stores the way a retarget does.
	fn apply_generation(world: &mut World, config: &Self::Config);
}

/// A mode's layout writes for base model `T`. Per-frame systems go in
/// [`crate::GenerationModeSystems<Self>`]; enter systems on
/// `OnEnter(ActiveGenerationMode::of::<Self>())`.
pub trait BaseTerrainScheme<T: TerrainGeneration>: GenerationMode {
	fn install(app: &mut App, config: &T::Config);
}

/// Shared install for `T`, added once. [`TerrainModel::require_generation`]
/// names this instead of a mode-specific plugin.
pub struct BaseTerrainGenerationCore<T: TerrainGeneration>(PhantomData<fn() -> T>);

impl<T: TerrainGeneration> Default for BaseTerrainGenerationCore<T> {
	fn default() -> Self {
		Self(PhantomData)
	}
}

impl<T: TerrainGeneration> Plugin for BaseTerrainGenerationCore<T> {
	fn build(&self, _app: &mut App) {}
}

/// Per-mode config the scheme systems read.
#[derive(Resource, Clone)]
pub struct BaseTerrainModeConfig<Mode: GenerationMode, T: TerrainGeneration> {
	pub config: T::Config,
	_mode: PhantomData<fn() -> Mode>,
}

impl<Mode: GenerationMode, T: TerrainGeneration> BaseTerrainModeConfig<Mode, T> {
	pub fn new(config: T::Config) -> Self {
		Self { config, _mode: PhantomData }
	}
}

/// Generation for base model `T` in `Mode`.
pub struct BaseTerrainGenerationPlugin<Mode, T>
where
	Mode: BaseTerrainScheme<T>,
	T: TerrainGeneration,
{
	pub config: T::Config,
	_mode: PhantomData<fn() -> Mode>,
}

impl<Mode, T> BaseTerrainGenerationPlugin<Mode, T>
where
	Mode: BaseTerrainScheme<T>,
	T: TerrainGeneration,
{
	pub fn new(config: T::Config) -> Self {
		Self { config, _mode: PhantomData }
	}
}

impl<Mode, T> Plugin for BaseTerrainGenerationPlugin<Mode, T>
where
	Mode: BaseTerrainScheme<T>,
	T: TerrainGeneration,
{
	fn build(&self, app: &mut App) {
		if !app.is_plugin_added::<BaseTerrainGenerationCore<T>>() {
			T::install_generation(app);
			app.add_plugins(BaseTerrainGenerationCore::<T>::default());
		}
		app.insert_resource(BaseTerrainModeConfig::<Mode, T>::new(self.config.clone()));
		app.add_systems(
			OnEnter(ActiveGenerationMode::of::<Mode>()),
			apply_base_terrain::<Mode, T>,
		);
		Mode::install(app, &self.config);
	}
}

fn apply_base_terrain<Mode, T>(world: &mut World)
where
	Mode: GenerationMode,
	T: TerrainGeneration,
{
	let config = world.resource::<BaseTerrainModeConfig<Mode, T>>().config.clone();
	T::apply_generation(world, &config);
}

#[cfg(test)]
mod tests {
	use super::*;
	use layer_stack::GenerationModePlugin;
	use crate::{HeightField, TerrainCell, TerrainModel};
	use layer_stack::RequireLayer;
	use bevy::ecs::system::{SystemParam, SystemParamItem};
	use bevy::prelude::NextState;
	use bevy::math::bounding::Aabb3d;
	use bevy::math::{Vec2, Vec3};
	use bevy::state::app::StatesPlugin;
	use bevy::transform::components::Transform;
	use lod::gen::Id;
	use lod::lod_ref::LodRef;

	struct Alpha;
	struct Beta;

	impl GenerationMode for Alpha {}
	impl GenerationMode for Beta {}

	#[derive(Resource, Default)]
	struct StubStore {
		installs: u32,
		applies: u32,
		seed: u32,
		fallback: f32,
	}

	struct Stub;

	#[derive(SystemParam)]
	struct StubRead<'w> {
		store: Res<'w, StubStore>,
	}

	impl TerrainCell for f32 {
		type Mesh = f32;
		fn bounds(&self) -> Aabb3d {
			Aabb3d::new(Vec3::ZERO, Vec3::ONE)
		}
		fn mesh_builder(&self) -> f32 {
			*self
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

	impl HeightField for f32 {
		fn height_at(&self, _xz: Vec2) -> Option<f32> {
			Some(*self)
		}
		fn fallback_height_at(&self, _xz: Vec2) -> f32 {
			*self
		}
	}

	impl TerrainModel for Stub {
		type Cell = f32;
		type Read = StubRead<'static>;
		type Snapshot = f32;
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

		fn fallback_height_at(read: &SystemParamItem<'_, '_, Self::Read>, _xz: Vec2) -> f32 {
			read.store.fallback
		}

		fn cell_ids_overlapping(
			_read: &SystemParamItem<'_, '_, Self::Read>,
			_region: Aabb3d,
		) -> Vec<Id> {
			Vec::new()
		}

		fn cell<'a>(
			_read: &'a SystemParamItem<'_, '_, Self::Read>,
			_id: Id,
		) -> Option<&'a f32> {
			None
		}

		fn overlay_cell<'a>(
			_read: &'a SystemParamItem<'_, '_, Self::Read>,
			_bounds: Aabb3d,
			_target_size: f32,
			_overlay_size_tolerance: Option<f32>,
		) -> Option<&'a dyn TerrainCell<Mesh = f32>> {
			None
		}

		fn snapshot(_read: &SystemParamItem<'_, '_, Self::Read>, _region: Aabb3d) -> f32 {
			0.0
		}

		fn require_generation(app: &App) {
			app.require_layer::<BaseTerrainGenerationCore<Stub>, Stub>();
		}
	}

	#[derive(Clone, Debug)]
	struct StubConfig {
		seed: u32,
		layout: f32,
	}

	impl TerrainGeneration for Stub {
		type Config = StubConfig;

		fn install_generation(app: &mut App) {
			let mut store = app.world_mut().get_resource_or_insert_with(StubStore::default);
			store.installs += 1;
		}

		fn apply_generation(world: &mut World, config: &Self::Config) {
			let mut store = world.resource_mut::<StubStore>();
			if store.seed != config.seed {
				store.seed = config.seed;
			}
			store.fallback = config.layout;
			store.applies += 1;
		}
	}

	impl BaseTerrainScheme<Stub> for Alpha {
		fn install(_app: &mut App, _config: &StubConfig) {}
	}

	impl BaseTerrainScheme<Stub> for Beta {
		fn install(_app: &mut App, _config: &StubConfig) {}
	}

	fn alpha_config() -> StubConfig {
		StubConfig { seed: 1, layout: 3.0 }
	}

	fn beta_config() -> StubConfig {
		StubConfig { seed: 2, layout: 9.0 }
	}

	fn hop(app: &mut App, mode: ActiveGenerationMode) {
		app.world_mut()
			.resource_mut::<NextState<ActiveGenerationMode>>()
			.set(mode);
		app.update();
	}

	fn live(app: &App) -> (u32, f32) {
		let store = app.world().resource::<StubStore>();
		(store.seed, store.fallback)
	}

	#[test]
	fn different_seeds_build_and_apply_on_enter() -> anyhow::Result<()> {
		let mut app = App::new();
		app.add_plugins((
			MinimalPlugins,
			StatesPlugin,
			GenerationModePlugin::<Alpha>::initial(),
			GenerationModePlugin::<Beta>::default(),
			BaseTerrainGenerationPlugin::<Alpha, Stub>::new(alpha_config()),
			BaseTerrainGenerationPlugin::<Beta, Stub>::new(beta_config()),
		));
		app.finish();
		let store = app.world().resource::<StubStore>();
		anyhow::ensure!(store.installs == 1, "shared install ran {}", store.installs);
		app.update();
		anyhow::ensure!(live(&app) == (1, 3.0), "initial mode applied {:?}", live(&app));

		hop(&mut app, ActiveGenerationMode::of::<Beta>());
		anyhow::ensure!(live(&app) == (2, 9.0), "beta seed and layout {:?}", live(&app));

		hop(&mut app, ActiveGenerationMode::of::<Alpha>());
		anyhow::ensure!(live(&app) == (1, 3.0), "return restores {:?}", live(&app));
		Ok(())
	}

	#[test]
	fn plugin_order_does_not_matter() -> anyhow::Result<()> {
		let mut generation_first = App::new();
		generation_first.add_plugins((
			MinimalPlugins,
			StatesPlugin,
			BaseTerrainGenerationPlugin::<Beta, Stub>::new(beta_config()),
			BaseTerrainGenerationPlugin::<Alpha, Stub>::new(alpha_config()),
			GenerationModePlugin::<Alpha>::initial(),
			GenerationModePlugin::<Beta>::default(),
		));
		generation_first.finish();
		generation_first.update();
		anyhow::ensure!(
			live(&generation_first) == (1, 3.0),
			"generation before mode still started from alpha: {:?}",
			live(&generation_first)
		);
		anyhow::ensure!(
			generation_first.is_plugin_added::<BaseTerrainGenerationCore<Stub>>(),
			"core is installed"
		);

		let mut beta_first = App::new();
		beta_first.add_plugins((
			MinimalPlugins,
			StatesPlugin,
			GenerationModePlugin::<Alpha>::initial(),
			GenerationModePlugin::<Beta>::default(),
			BaseTerrainGenerationPlugin::<Beta, Stub>::new(beta_config()),
			BaseTerrainGenerationPlugin::<Alpha, Stub>::new(alpha_config()),
		));
		beta_first.finish();
		beta_first.update();
		anyhow::ensure!(
			live(&beta_first) == (1, 3.0),
			"beta plugin first still started from alpha: {:?}",
			live(&beta_first)
		);
		Ok(())
	}
}
