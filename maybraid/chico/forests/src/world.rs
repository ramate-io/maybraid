//! Composed world sources for forest present.
//!
//! Hide / cull / spawn stay on [`ForestPresenter`](crate::present::ForestPresenter).
//! `S` only marshals a [`GroveWorldSample`] from the gen stack.

use bevy::ecs::system::{Local, StaticSystemParam, SystemParam};
use bevy::math::bounding::Aabb3d;
use lod::lod_ref::LodRef;

use crate::grove::ChicoGrove;
use crate::index::forest_world_sample;
use chico_groves::GroveWorldSample;

/// Assembles the sample groves grow against for one present handle.
pub trait GroveWorldSource {
	fn sample(
		&mut self,
		grove: &ChicoGrove,
		lod_ref: &LodRef,
	) -> Option<impl GroveWorldSample + Clone + Send + Sync + 'static>;
}

/// Height field that [`OnTerrain`] can seek and snapshot.
pub trait TerrainHeightSource {
	fn ensure_and_sample(
		&mut self,
		bounds: Aabb3d,
		lod_ref: &LodRef,
	) -> Option<impl GroveWorldSample + Clone + Send + Sync + 'static>;
}

/// Flat ground. Isolation tests and hosts with no terrain model.
#[derive(SystemParam)]
pub struct FlatWorld<'s> {
	_local: Local<'s, ()>,
}

impl GroveWorldSource for FlatWorld<'_> {
	fn sample(
		&mut self,
		_grove: &ChicoGrove,
		_lod_ref: &LodRef,
	) -> Option<impl GroveWorldSample + Clone + Send + Sync + 'static> {
		Some(forest_world_sample())
	}
}

/// Seek a terrain height model in the gen stack, then snapshot it for grow.
#[derive(SystemParam)]
pub struct OnTerrain<'w, 's, H: SystemParam + 'static> {
	height: StaticSystemParam<'w, 's, H>,
}

impl<H: SystemParam + 'static> GroveWorldSource for OnTerrain<'_, '_, H>
where
	for<'a, 'b> H::Item<'a, 'b>: TerrainHeightSource,
{
	fn sample(
		&mut self,
		grove: &ChicoGrove,
		lod_ref: &LodRef,
	) -> Option<impl GroveWorldSample + Clone + Send + Sync + 'static> {
		self.height.ensure_and_sample(grove.aabb(), lod_ref)
	}
}
