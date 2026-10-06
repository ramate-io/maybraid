//! [`Built`]: the hosts fitted to a filled development.

use std::marker::PhantomData;

use bevy::math::bounding::Aabb3d;
use lod::gen::{GenerationScheme, Id, OriginalId};
use lod::hcsg::HcsgStorage;

use crate::artifact::BuiltDevelopment;
use crate::developments::site::DevelopmentSite;
use crate::developments::RichmondDevelopment;
use crate::ground::RichmondGround;
use crate::storage::column_bounds;

/// Buildings fitted to one filled [`RichmondDevelopment`] over ground `G`.
pub struct Built<G> {
	pub development: BuiltDevelopment,
	_ground: PhantomData<fn() -> G>,
}

impl<G> Built<G> {
	pub fn new(development: BuiltDevelopment) -> Self {
		Self { development, _ground: PhantomData }
	}
}

impl<G: RichmondGround> GenerationScheme<HcsgStorage> for Built<G> {
	fn original_ids_for(storage: &mut HcsgStorage, region: Aabb3d) -> Vec<OriginalId> {
		storage
			.original_ids_for::<RichmondDevelopment<G>>(region)
			.into_iter()
			.filter(|OriginalId(id)| {
				storage
					.get_one_or_generate::<RichmondDevelopment<G>>(*id)
					.is_some_and(RichmondDevelopment::is_filled)
			})
			.collect()
	}

	/// Fitted with the site's noise seed: the authored config, else the global one.
	fn build_with_id(storage: &mut HcsgStorage, id: Id) -> Option<(Self, Aabb3d)> {
		let seed = storage.get_one_or_generate::<DevelopmentSite>(id)?.clone().config(storage)?.seed;
		let development = storage.get_one_or_generate::<RichmondDevelopment<G>>(id)?;
		let cell = development.cell();
		let built = development.built(seed as i32)?;
		Some((Self::new(built), column_bounds(cell)))
	}
}
