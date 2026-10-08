//! [`ShepherdsVillageCell`]: a [`ShepherdsVillage`] laid out over ground `G`.

use std::marker::PhantomData;

use bevy::math::bounding::Aabb3d;
use lod::gen::{GenerationScheme, Id, OriginalId};
use lod::hcsg::HcsgStorage;
use urbanization_developments::ShepherdsVillage;

use super::site::{DevelopmentKind, DevelopmentSite};
use super::DevelopmentPad;
use crate::artifact::BuiltDevelopment;
use crate::ground::{GroundSampler, RichmondGround};
use crate::storage::column_bounds;

/// A Shepherds Village over ground `G`: one pad per hut or house.
pub struct ShepherdsVillageCell<G> {
	pub cell: Aabb3d,
	pub pads: Vec<DevelopmentPad>,
	pub village: ShepherdsVillage,
	_ground: PhantomData<fn() -> G>,
}

impl<G> ShepherdsVillageCell<G> {
	pub fn built(&self) -> BuiltDevelopment {
		BuiltDevelopment::ShepherdsVillage(Box::new(self.village.clone()))
	}
}

impl<G: RichmondGround> GenerationScheme<HcsgStorage> for ShepherdsVillageCell<G> {
	fn original_ids_for(storage: &mut HcsgStorage, region: Aabb3d) -> Vec<OriginalId> {
		DevelopmentSite::ids_of_kind(storage, region, DevelopmentKind::ShepherdsVillage)
	}

	fn build_with_id(storage: &mut HcsgStorage, id: Id) -> Option<(Self, Aabb3d)> {
		let (site, config) =
			DevelopmentSite::planned(storage, id, DevelopmentKind::ShepherdsVillage)?;
		let bounds = column_bounds(site.cell);
		let mut ground = GroundSampler::<G>::new(storage, bounds);
		let (village, pads) = ShepherdsVillage::lay_out(&mut ground, site.cell, config.seed)?;
		let pads = pads.iter().map(DevelopmentPad::from).collect();
		Some((Self { cell: site.cell, pads, village, _ground: PhantomData }, bounds))
	}
}
