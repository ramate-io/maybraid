//! Base of the stack: models that own a generation pipeline.

use std::any::type_name;
use std::fmt::Debug;
use std::marker::PhantomData;

use bevy::app::{App, Plugin};
use bevy::prelude::*;

use crate::generation_mode::{ActiveGenerationMode, GenerationMode};
use crate::model::TerrainModel;

/// A model with its own generation stack (the bottom of the wiring diagram).
///
/// Wrappers such as `Urbanization<M>` are not `TerrainGeneration`; their
/// generation plugins are typed by the model they read instead.
pub trait TerrainGeneration: TerrainModel {
	type Config: Clone + Send + Sync + 'static;
	/// The part of [`Self::Config`] every mode must agree on.
	type SharedConfig: Clone + PartialEq + Debug + Send + Sync + 'static;

	fn shared_config(config: &Self::Config) -> Self::SharedConfig;

	/// Register the model's stores, layout, and generate systems. No presentation.
	fn install_generation(app: &mut App, config: &Self::Config);
}

/// A mode's layout writes for base model `T`. Per-frame systems go in
/// [`crate::GenerationModeSystems<Self>`]; enter systems on
/// `OnEnter(ActiveGenerationMode::of::<Self>())`.
pub trait BaseTerrainScheme<T: TerrainGeneration>: GenerationMode {
	fn install(app: &mut App, config: &T::Config);
}

/// Shared install for `T`, added once. [`TerrainModel::require_generation`]
/// names this instead of a mode-specific plugin.
pub struct BaseTerrainGenerationCore<T: TerrainGeneration> {
	pub shared: T::SharedConfig,
}

impl<T: TerrainGeneration> Plugin for BaseTerrainGenerationCore<T> {
	fn build(&self, app: &mut App) {
		app.insert_resource(InstalledBaseTerrainShared::<T>(self.shared.clone()));
	}
}

#[derive(Resource)]
struct InstalledBaseTerrainShared<T: TerrainGeneration>(T::SharedConfig);

#[derive(Resource)]
struct PendingBaseTerrainShared<T: TerrainGeneration> {
	shared: Vec<T::SharedConfig>,
}

/// Set when the initial mode's plugin supplies `T`'s startup config.
#[derive(Resource)]
struct BaseTerrainStartupInstalled<T: TerrainGeneration>(PhantomData<T>);

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
		let Some(state) = app.world().get_resource::<State<ActiveGenerationMode>>() else {
			panic!(
				"BaseTerrainGenerationPlugin<{}, {}> requires GenerationModePlugin first",
				Mode::name(),
				type_name::<T>()
			);
		};
		let startup = state.get().is::<Mode>();
		let shared = T::shared_config(&self.config);
		if let Some(installed) = app.world().get_resource::<InstalledBaseTerrainShared<T>>() {
			require_shared_agrees::<T>(&installed.0, &shared);
		} else if startup {
			require_pending_agrees::<T>(app, &shared);
			T::install_generation(app, &self.config);
			app.add_plugins(BaseTerrainGenerationCore::<T> { shared: shared.clone() })
				.insert_resource(BaseTerrainStartupInstalled::<T>(PhantomData));
			app.world_mut().remove_resource::<PendingBaseTerrainShared<T>>();
		} else {
			app.world_mut()
				.get_resource_or_insert_with(|| PendingBaseTerrainShared::<T> {
					shared: Vec::new(),
				})
				.shared
				.push(shared);
		}
		Mode::install(app, &self.config);
	}

	fn finish(&self, app: &mut App) {
		if !app.world().contains_resource::<BaseTerrainStartupInstalled<T>>() {
			panic!(
				"the initial generation mode never registered BaseTerrainGenerationPlugin for {}",
				type_name::<T>()
			);
		}
	}
}

fn require_shared_agrees<T: TerrainGeneration>(installed: &T::SharedConfig, shared: &T::SharedConfig) {
	if installed != shared {
		panic!(
			"BaseTerrainGenerationPlugin shared config disagrees for {}: {installed:?} vs {shared:?}",
			type_name::<T>()
		);
	}
}

fn require_pending_agrees<T: TerrainGeneration>(app: &App, shared: &T::SharedConfig) {
	let Some(pending) = app.world().get_resource::<PendingBaseTerrainShared<T>>() else {
		return;
	};
	for other in &pending.shared {
		require_shared_agrees::<T>(other, shared);
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::generation_mode::GenerationModePlugin;
	use crate::{HeightField, RequireLayer, TerrainCell, TerrainModel};
	use bevy::ecs::system::{SystemParam, SystemParamItem};
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
		type SharedConfig = u32;

		fn shared_config(config: &Self::Config) -> u32 {
			config.seed
		}

		fn install_generation(app: &mut App, config: &Self::Config) {
			let mut store = app.world_mut().get_resource_or_insert_with(StubStore::default);
			store.installs += 1;
			store.fallback = config.layout;
		}
	}

	impl BaseTerrainScheme<Stub> for Alpha {
		fn install(_app: &mut App, _config: &StubConfig) {}
	}

	impl BaseTerrainScheme<Stub> for Beta {
		fn install(_app: &mut App, _config: &StubConfig) {}
	}

	fn alpha_config(layout: f32) -> StubConfig {
		StubConfig { seed: 1, layout }
	}

	fn beta_config(layout: f32) -> StubConfig {
		StubConfig { seed: 1, layout }
	}

	#[test]
	fn two_modes_install_the_shared_part_once() -> anyhow::Result<()> {
		let mut app = App::new();
		app.add_plugins((
			MinimalPlugins,
			StatesPlugin,
			GenerationModePlugin::<Alpha>::initial(),
			GenerationModePlugin::<Beta>::default(),
			BaseTerrainGenerationPlugin::<Alpha, Stub>::new(alpha_config(3.0)),
			BaseTerrainGenerationPlugin::<Beta, Stub>::new(beta_config(9.0)),
		));
		app.finish();
		let store = app.world().resource::<StubStore>();
		anyhow::ensure!(store.installs == 1, "shared install ran {}", store.installs);
		anyhow::ensure!(
			store.fallback == 3.0,
			"startup used the initial mode, fallback {}",
			store.fallback
		);
		Ok(())
	}

	#[test]
	fn startup_uses_the_initial_mode_in_either_plugin_order() -> anyhow::Result<()> {
		let mut after_beta = App::new();
		after_beta.add_plugins((
			MinimalPlugins,
			StatesPlugin,
			GenerationModePlugin::<Alpha>::initial(),
			GenerationModePlugin::<Beta>::default(),
			BaseTerrainGenerationPlugin::<Beta, Stub>::new(beta_config(9.0)),
			BaseTerrainGenerationPlugin::<Alpha, Stub>::new(alpha_config(3.0)),
		));
		after_beta.finish();
		anyhow::ensure!(
			after_beta.world().resource::<StubStore>().fallback == 3.0,
			"beta first still started from alpha"
		);

		let mut after_alpha = App::new();
		after_alpha.add_plugins((
			MinimalPlugins,
			StatesPlugin,
			GenerationModePlugin::<Alpha>::initial(),
			GenerationModePlugin::<Beta>::default(),
			BaseTerrainGenerationPlugin::<Alpha, Stub>::new(alpha_config(3.0)),
			BaseTerrainGenerationPlugin::<Beta, Stub>::new(beta_config(9.0)),
		));
		after_alpha.finish();
		anyhow::ensure!(
			after_alpha.world().resource::<StubStore>().fallback == 3.0,
			"alpha first started from alpha"
		);
		Ok(())
	}

	#[test]
	fn a_disagreeing_shared_config_fails() -> anyhow::Result<()> {
		let failed = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
			let mut app = App::new();
			app.add_plugins((
				MinimalPlugins,
				StatesPlugin,
				GenerationModePlugin::<Alpha>::initial(),
				GenerationModePlugin::<Beta>::default(),
				BaseTerrainGenerationPlugin::<Alpha, Stub>::new(alpha_config(3.0)),
				BaseTerrainGenerationPlugin::<Beta, Stub>::new(StubConfig { seed: 2, layout: 9.0 }),
			));
		}));
		anyhow::ensure!(failed.is_err(), "disagreeing seeds must fail loudly");
		Ok(())
	}

	#[test]
	fn missing_generation_mode_plugin_fails_at_build() -> anyhow::Result<()> {
		let failed = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
			let mut app = App::new();
			app.add_plugins((
				MinimalPlugins,
				StatesPlugin,
				BaseTerrainGenerationPlugin::<Alpha, Stub>::new(alpha_config(3.0)),
			));
		}));
		anyhow::ensure!(failed.is_err(), "base terrain requires GenerationModePlugin first");
		Ok(())
	}

	#[test]
	fn an_initial_mode_without_base_terrain_fails_in_finish() -> anyhow::Result<()> {
		let failed = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
			let mut app = App::new();
			app.add_plugins((
				MinimalPlugins,
				StatesPlugin,
				GenerationModePlugin::<Alpha>::initial(),
				GenerationModePlugin::<Beta>::default(),
				BaseTerrainGenerationPlugin::<Beta, Stub>::new(beta_config(9.0)),
			));
			app.finish();
		}));
		anyhow::ensure!(failed.is_err(), "finish must require the initial mode's plugin");
		Ok(())
	}
}
