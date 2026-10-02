//! [`Urbanization`] wrapper and its [`TerrainModel`] impl.

use std::marker::PhantomData;

use bevy::ecs::system::{StaticSystemParam, SystemParam, SystemParamItem};
use bevy::math::bounding::Aabb3d;
use bevy::math::Vec2;
use bevy::prelude::App;
use lod::lod_ref::LodRef;
use terrain_layer_model::{HeightField, TerrainCell, TerrainModel};

use crate::pads::PadOps;
use crate::urban::{UrbanModel, UrbanizationModel};

/// Model `U` after urbanization: pads composed into `U::Ground`, urban artifacts on it.
pub struct Urbanization<U>(PhantomData<fn() -> U>);

/// Ground read plus the urbanization model's own resources.
#[derive(SystemParam)]
pub struct UrbanRead<'w, 's, U: UrbanizationModel> {
	pub ground: StaticSystemParam<'w, 's, <<U as UrbanizationModel>::Ground as TerrainModel>::Read>,
	pub urban: StaticSystemParam<'w, 's, <U as UrbanizationModel>::Read>,
}

/// Inner snapshot plus the pads merged over the snapshot region.
#[derive(Clone)]
pub struct UrbanSnapshot<S, P> {
	inner: S,
	pads: P,
}

impl<S, P> UrbanSnapshot<S, P> {
	pub fn new(inner: S, pads: P) -> Self {
		Self { inner, pads }
	}
}

impl<S: HeightField, P: PadOps> HeightField for UrbanSnapshot<S, P> {
	fn height_at(&self, xz: Vec2) -> Option<f32> {
		self.inner.height_at(xz).map(|raw| self.pads.modify_elevation(raw, xz.x, xz.y))
	}

	fn fallback_height_at(&self, xz: Vec2) -> f32 {
		let raw = self.inner.fallback_height_at(xz);
		self.pads.modify_elevation(raw, xz.x, xz.y)
	}
}

impl<U> TerrainModel for Urbanization<U>
where
	U: UrbanizationModel,
	<U::Ground as TerrainModel>::Cell: TerrainCell<Mesh = <U::Surface as TerrainCell>::Mesh>,
{
	type Base = <U::Ground as TerrainModel>::Base;
	type Cell = U::Surface;
	type Read = UrbanRead<'static, 'static, U>;
	type Snapshot = UrbanSnapshot<<U::Ground as TerrainModel>::Snapshot, U::Pads>;
	type Prepare = U::Prepare;

	/// #720 wart: generate development cells for `bounds` before the grove sample.
	fn prepare(
		prepare: &mut SystemParamItem<'_, '_, Self::Prepare>,
		bounds: Aabb3d,
		lod_ref: &LodRef,
	) {
		U::prepare(prepare, bounds, lod_ref);
	}

	fn height_at(read: &SystemParamItem<'_, '_, Self::Read>, xz: Vec2) -> Option<f32> {
		let raw = U::Ground::height_at(&read.ground, xz)?;
		Some(U::pads_at(&read.urban, xz).modify_elevation(raw, xz.x, xz.y))
	}

	fn fallback_height_at(read: &SystemParamItem<'_, '_, Self::Read>, xz: Vec2) -> f32 {
		let raw = U::Ground::fallback_height_at(&read.ground, xz);
		U::pads_at(&read.urban, xz).modify_elevation(raw, xz.x, xz.y)
	}

	/// Padded cell when its size passes `overlay_size_tolerance`, else the
	/// inner model's raw cell.
	fn overlay_cell<'a>(
		read: &'a SystemParamItem<'_, '_, Self::Read>,
		bounds: Aabb3d,
		target_size: f32,
		overlay_size_tolerance: Option<f32>,
	) -> Option<&'a dyn TerrainCell<Mesh = <Self::Cell as TerrainCell>::Mesh>> {
		if let Some(padded) = U::overlay_surface(&read.urban, bounds) {
			let size = padded.bounds().max.x - padded.bounds().min.x;
			let accept = match overlay_size_tolerance {
				None => true,
				Some(tolerance) => (size - target_size).abs() < tolerance,
			};
			if accept {
				return Some(padded);
			}
		}
		U::Ground::overlay_cell(&read.ground, bounds, target_size, overlay_size_tolerance)
	}

	fn snapshot(read: &SystemParamItem<'_, '_, Self::Read>, region: Aabb3d) -> Self::Snapshot {
		UrbanSnapshot::new(U::Ground::snapshot(&read.ground, region), U::pads(&read.urban, region))
	}

	fn require_generation(app: &App) {
		U::Ground::require_generation(app);
		U::require_generation(app);
	}
}

impl<U> UrbanModel for Urbanization<U>
where
	U: UrbanizationModel,
	<U::Ground as TerrainModel>::Cell: TerrainCell<Mesh = <U::Surface as TerrainCell>::Mesh>,
{
	type Built = U::Built;
	type Pads = U::Pads;

	fn pads(read: &SystemParamItem<'_, '_, Self::Read>, region: Aabb3d) -> U::Pads {
		U::pads(&read.urban, region)
	}

	fn built<'a>(
		read: &'a SystemParamItem<'_, '_, Self::Read>,
		region: Aabb3d,
	) -> Vec<&'a U::Built> {
		U::built(&read.urban, region)
	}

	fn built_overlapping<'a>(
		read: &'a SystemParamItem<'_, '_, Self::Read>,
		region: Aabb3d,
	) -> Vec<(lod::gen::Id, lod::gen::Version, &'a U::Built)> {
		U::built_overlapping(&read.urban, region)
	}
}
