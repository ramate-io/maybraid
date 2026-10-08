//! [`OldCityMarketCell`]: an [`OldCityMarket`] laid out over ground `G`.

use std::marker::PhantomData;

use bevy::math::bounding::Aabb3d;
use lod::gen::{GenerationScheme, Id, OriginalId};
use lod::hcsg::HcsgStorage;
use urbanization_developments::OldCityMarket;

use super::site::{DevelopmentKind, DevelopmentSite};
use super::DevelopmentPad;
use crate::artifact::BuiltDevelopment;
use crate::ground::{GroundSampler, RichmondGround};
use crate::storage::column_bounds;

/// An Old City Market over ground `G`: terraces and lanes as separate pads.
pub struct OldCityMarketCell<G> {
	pub cell: Aabb3d,
	pub pads: Vec<DevelopmentPad>,
	pub market: OldCityMarket,
	_ground: PhantomData<fn() -> G>,
}

impl<G> OldCityMarketCell<G> {
	pub fn built(&self) -> BuiltDevelopment {
		BuiltDevelopment::OldCityMarket(Box::new(self.market.clone()))
	}
}

impl<G: RichmondGround> GenerationScheme<HcsgStorage> for OldCityMarketCell<G> {
	fn original_ids_for(storage: &mut HcsgStorage, region: Aabb3d) -> Vec<OriginalId> {
		DevelopmentSite::ids_of_kind(storage, region, DevelopmentKind::OldCityMarket)
	}

	fn build_with_id(storage: &mut HcsgStorage, id: Id) -> Option<(Self, Aabb3d)> {
		let (site, config) = DevelopmentSite::planned(storage, id, DevelopmentKind::OldCityMarket)?;
		let bounds = column_bounds(site.cell);
		let mut ground = GroundSampler::<G>::new(storage, bounds);
		let (market, pads) = OldCityMarket::lay_out(&mut ground, site.cell, config.seed)?;
		let pads = pads.iter().map(DevelopmentPad::from).collect();
		Some((Self { cell: site.cell, pads, market, _ground: PhantomData }, bounds))
	}
}

#[cfg(test)]
mod tests {
	use urbanization_developments::{PadPlan, SiteGround};

	use super::*;
	use crate::cell::DevelopmentExtent;
	use crate::DevelopmentHosts;

	struct DryFlat;

	impl SiteGround for DryFlat {
		fn height_at(&mut self, _x: f32, _z: f32) -> Option<f32> {
			Some(12.0)
		}

		fn hydro_overlaps(&mut self, _pad: &PadPlan) -> bool {
			false
		}
	}

	#[test]
	fn market_hosts_every_stall_and_terrace() -> anyhow::Result<()> {
		let cell = DevelopmentExtent::from_cell_index(0, 0).aabb();
		let market = (0..16)
			.find_map(|seed| OldCityMarket::lay_out(&mut DryFlat, cell, seed))
			.map(|(market, _)| market)
			.ok_or_else(|| anyhow::anyhow!("a market should fit on flat dry ground"))?;
		let hosts = BuiltDevelopment::OldCityMarket(Box::new(market.clone())).hosts();
		assert_eq!(hosts.len(), market.stall_count() + market.nodes.len());
		Ok(())
	}
}
