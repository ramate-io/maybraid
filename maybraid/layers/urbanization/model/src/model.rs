//! [`Urbanization`] marker and its [`TerrainModel`] impl.

use std::marker::PhantomData;

use bevy::ecs::system::{Res, StaticSystemParam, SystemParam, SystemParamItem};
use bevy::math::bounding::Aabb3d;
use bevy::math::{Vec2, Vec3};
use bevy::prelude::App;
use lod::gen::{Id, SpatialIndex, TrackedId};
use lod::lod_ref::LodRef;
use durham_terrain_models::TerrainMeshBuilder;
use richmond_development_models::{
	DevelopmentEntryStore, DevelopmentIndex, PadComplex, PaddedStoreView, TerrainWithPads,
};
use richmond_urbanization::UrbanizationIndex;
use terrain_layer_model::{HeightField, RequireLayer, TerrainCell, TerrainModel};

use crate::generation::UrbanizationGenerationPlugin;
use crate::pads::PadComposable;
use crate::stream::prepare_development_cells;

/// Model `M` after urbanization: pads composed into its surface, developments on it.
pub struct Urbanization<M>(PhantomData<fn() -> M>);

/// Resources behind `Urbanization<M>`'s [`TerrainModel::Read`].
#[derive(SystemParam)]
pub struct UrbanRead<'w, 's, M: TerrainModel> {
	pub(crate) inner: StaticSystemParam<'w, 's, <M as TerrainModel>::Read>,
	pub(crate) developments: Res<'w, DevelopmentEntryStore>,
	pub(crate) urbanization: Res<'w, UrbanizationIndex>,
}

impl<M: TerrainModel> UrbanRead<'_, '_, M> {
	/// Merged pads under one XZ point (the probe world player / mobs use today).
	pub(crate) fn pads_at(&self, xz: Vec2) -> PadComplex {
		self.developments.merged_pad_complex(Aabb3d::from_min_max(
			Vec3::new(xz.x - 0.5, -10_000.0, xz.y - 0.5),
			Vec3::new(xz.x + 0.5, 10_000.0, xz.y + 0.5),
		))
	}
}

/// Inner snapshot plus the pads merged over the snapshot region.
#[derive(Clone)]
pub struct UrbanSnapshot<S> {
	inner: S,
	pads: PadComplex,
}

impl<S> UrbanSnapshot<S> {
	pub fn new(inner: S, pads: PadComplex) -> Self {
		Self { inner, pads }
	}
}

impl<S: HeightField> HeightField for UrbanSnapshot<S> {
	fn height_at(&self, xz: Vec2) -> Option<f32> {
		self.inner.height_at(xz).map(|raw| self.pads.modify_elevation(raw, xz.x, xz.y))
	}

	fn fallback_height_at(&self, xz: Vec2) -> f32 {
		let raw = self.inner.fallback_height_at(xz);
		self.pads.modify_elevation(raw, xz.x, xz.y)
	}
}

impl<M> TerrainModel for Urbanization<M>
where
	M: TerrainModel,
	M::Cell: PadComposable<Padded = TerrainWithPads> + TerrainCell<Mesh = TerrainMeshBuilder>,
{
	type Cell = TerrainWithPads;
	type Read = UrbanRead<'static, 'static, M>;
	type Snapshot = UrbanSnapshot<M::Snapshot>;
	type Prepare = DevelopmentIndex<'static>;

	/// #720 wart: generate development cells for `bounds` before the grove sample.
	fn prepare(
		prepare: &mut SystemParamItem<'_, '_, Self::Prepare>,
		bounds: Aabb3d,
		lod_ref: &LodRef,
	) {
		prepare_development_cells(prepare, bounds, lod_ref);
	}

	/// Inner height with pad elevation ops, the formula world player and mobs
	/// copy today. Padded cell SDFs agree when fresh; they are for meshing.
	fn height_at(read: &SystemParamItem<'_, '_, Self::Read>, xz: Vec2) -> Option<f32> {
		let raw = M::height_at(&read.inner, xz)?;
		Some(read.pads_at(xz).modify_elevation(raw, xz.x, xz.y))
	}

	fn fallback_height_at(read: &SystemParamItem<'_, '_, Self::Read>, xz: Vec2) -> f32 {
		let raw = M::fallback_height_at(&read.inner, xz);
		read.pads_at(xz).modify_elevation(raw, xz.x, xz.y)
	}

	fn cell_ids_overlapping(read: &SystemParamItem<'_, '_, Self::Read>, region: Aabb3d) -> Vec<Id> {
		PaddedStoreView::new(&read.developments)
			.tracked_ids_for(region)
			.into_iter()
			.map(|TrackedId(id)| id)
			.collect()
	}

	fn cell<'a>(
		read: &'a SystemParamItem<'_, '_, Self::Read>,
		id: Id,
	) -> Option<&'a TerrainWithPads> {
		read.developments.padded(id)
	}

	/// Padded cell when its size passes `overlay_size_tolerance`, else the inner
	/// model's raw cell (`fine_terrain_for` / `medium_terrain_for`).
	fn overlay_cell<'a>(
		read: &'a SystemParamItem<'_, '_, Self::Read>,
		bounds: Aabb3d,
		target_size: f32,
		overlay_size_tolerance: Option<f32>,
	) -> Option<&'a dyn TerrainCell<Mesh = TerrainMeshBuilder>> {
		if let Some(padded) = read.developments.padded_terrain_for(bounds) {
			let size = padded.bounds().max.x - padded.bounds().min.x;
			let accept = match overlay_size_tolerance {
				None => true,
				Some(tolerance) => (size - target_size).abs() < tolerance,
			};
			if accept {
				return Some(padded);
			}
		}
		M::overlay_cell(&read.inner, bounds, target_size, overlay_size_tolerance)
	}

	fn snapshot(read: &SystemParamItem<'_, '_, Self::Read>, region: Aabb3d) -> Self::Snapshot {
		UrbanSnapshot::new(
			M::snapshot(&read.inner, region),
			read.developments.merged_pad_complex(region),
		)
	}

	fn require_generation(app: &App) {
		M::require_generation(app);
		app.require_layer::<UrbanizationGenerationPlugin<M>, Self>();
	}
}
