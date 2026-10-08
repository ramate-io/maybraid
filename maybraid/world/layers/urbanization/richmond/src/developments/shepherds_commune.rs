//! [`ShepherdsCommuneCell`]: a [`ShepherdsCommune`] laid out over ground `G`.

use std::marker::PhantomData;

use bevy::math::bounding::Aabb3d;
use lod::gen::{GenerationScheme, Id, OriginalId};
use lod::hcsg::HcsgStorage;
use urbanization_developments::ShepherdsCommune;

use super::site::{DevelopmentKind, DevelopmentSite};
use super::DevelopmentPad;
use crate::artifact::BuiltDevelopment;
use crate::ground::{GroundSampler, RichmondGround};
use crate::storage::column_bounds;

/// A Shepherds Commune over ground `G`: graded corridors joining building pads.
pub struct ShepherdsCommuneCell<G> {
	pub cell: Aabb3d,
	pub pads: Vec<DevelopmentPad>,
	pub commune: ShepherdsCommune,
	_ground: PhantomData<fn() -> G>,
}

impl<G> ShepherdsCommuneCell<G> {
	pub fn built(&self) -> BuiltDevelopment {
		BuiltDevelopment::ShepherdsCommune(Box::new(self.commune.clone()))
	}
}

impl<G: RichmondGround> GenerationScheme<HcsgStorage> for ShepherdsCommuneCell<G> {
	fn original_ids_for(storage: &mut HcsgStorage, region: Aabb3d) -> Vec<OriginalId> {
		DevelopmentSite::ids_of_kind(storage, region, DevelopmentKind::ShepherdsCommune)
	}

	fn build_with_id(storage: &mut HcsgStorage, id: Id) -> Option<(Self, Aabb3d)> {
		let (site, config) =
			DevelopmentSite::planned(storage, id, DevelopmentKind::ShepherdsCommune)?;
		let bounds = column_bounds(site.cell);
		let mut ground = GroundSampler::<G>::new(storage, bounds);
		let (commune, pads) = ShepherdsCommune::lay_out(&mut ground, site.cell, config.seed)?;
		let pads = pads.iter().map(DevelopmentPad::from).collect();
		Some((Self { cell: site.cell, pads, commune, _ground: PhantomData }, bounds))
	}
}
