//! [`Mobs`] wrapper and its [`TerrainModel`] impl.

use std::marker::PhantomData;

use bevy::ecs::system::SystemParamItem;
use bevy::math::bounding::Aabb3d;
use bevy::math::Vec2;
use bevy::prelude::{App, Update, World};
use layer_stack::{Layer, LayerPresentation, LayerSystems};
use lod::lod_ref::LodRef;
use terrain_layer_model::{TerrainCell, TerrainModel};

use crate::generation::{MobGeneration, MobGenerationSystems};
use crate::mob::MobModel;

/// Model `B` after mobs: the same heights as `B::Ground`.
pub struct Mobs<B>(PhantomData<fn() -> B>);

impl<B> TerrainModel for Mobs<B>
where
	B: MobModel,
{
	type Base = <B::Ground as TerrainModel>::Base;
	type Cell = <B::Ground as TerrainModel>::Cell;
	type Read = <B::Ground as TerrainModel>::Read;
	type Snapshot = <B::Ground as TerrainModel>::Snapshot;
	type Prepare = <B::Ground as TerrainModel>::Prepare;

	fn prepare(
		prepare: &mut SystemParamItem<'_, '_, Self::Prepare>,
		bounds: Aabb3d,
		lod_ref: &LodRef,
	) {
		B::Ground::prepare(prepare, bounds, lod_ref);
	}

	fn height_at(read: &SystemParamItem<'_, '_, Self::Read>, xz: Vec2) -> Option<f32> {
		B::Ground::height_at(read, xz)
	}

	fn fallback_height_at(read: &SystemParamItem<'_, '_, Self::Read>, xz: Vec2) -> f32 {
		B::Ground::fallback_height_at(read, xz)
	}

	fn overlay_cell<'a>(
		read: &'a SystemParamItem<'_, '_, Self::Read>,
		bounds: Aabb3d,
		target_size: f32,
		overlay_size_tolerance: Option<f32>,
	) -> Option<&'a dyn TerrainCell<Mesh = <Self::Cell as TerrainCell>::Mesh>> {
		B::Ground::overlay_cell(read, bounds, target_size, overlay_size_tolerance)
	}

	fn snapshot(read: &SystemParamItem<'_, '_, Self::Read>, region: Aabb3d) -> Self::Snapshot {
		B::Ground::snapshot(read, region)
	}

	fn require_generation(app: &App) {
		B::Ground::require_generation(app);
		B::require_generation(app);
	}
}

impl<B: MobGeneration> Layer for Mobs<B> {
	const LABEL: &'static str = B::LABEL;
	type Config = B::Config;

	fn install_generation(app: &mut App) {
		B::install_generation(app);
		app.configure_sets(Update, MobGenerationSystems);
		app.configure_sets(Update, LayerSystems::<Mobs<B>>::default());
	}

	fn apply_generation(world: &mut World, config: &Self::Config) {
		B::apply_generation(world, config);
	}

	fn clear_generation(world: &mut World) {
		B::clear_generation(world);
	}

	fn require_lower(app: &App) {
		B::Ground::require_generation(app);
	}
}

impl<B: MobGeneration> LayerPresentation for Mobs<B> {
	fn install_presentation(app: &mut App) {
		B::install_presentation(app);
	}
}
