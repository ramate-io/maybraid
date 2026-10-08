//! [`RichmondDevelopment`]: the development a [`DevelopmentSite`] gets.
//!
//! Every [`Development`] in [`urbanization_developments`] generates the same
//! way, as a [`DevelopmentCell`] over ground `G`; the enum generates the site,
//! then the first of its kinds that fits.

pub mod site;

use std::marker::PhantomData;

use bevy::math::bounding::Aabb3d;
use bevy::math::Vec2;
use lod::gen::{GenerationScheme, Id, OriginalId};
use lod::hcsg::HcsgStorage;
use procedural_common::Bounds2;
use urbanization_developments::{
	Development, LesHalles, OldCityMarket, PadParams, PadPlan, RingFort, ShepherdsCommune,
	ShepherdsVillage, SingleHighrise, SiteGround, SkybridgeBazaar, SuburbanHomes, TempleComplex,
	Terrace, WizardsTower,
};

use crate::artifact::BuiltDevelopment;
use crate::ground::{GroundSampler, RichmondGround};
use crate::pad::{cell_center_xz, PadComplex};
use crate::storage::{column_bounds, overlaps_xz_strictly};

use site::{AuthoredCourtyard, DevelopmentKind, DevelopmentSite};

/// Pad baked from a post-Watershed height sample: flatten terrace + ease skirt.
#[derive(Debug, Clone)]
pub struct DevelopmentPad {
	pub height: f32,
	pub complex: PadComplex,
}

impl From<&PadPlan> for DevelopmentPad {
	fn from(plan: &PadPlan) -> Self {
		Self { height: plan.height, complex: PadComplex::from(plan) }
	}
}

/// A [`Development`] a [`DevelopmentSite`] can be.
pub trait SiteDevelopment: Development {
	const KIND: DevelopmentKind;

	fn into_built(self) -> BuiltDevelopment;
}

macro_rules! site_developments {
	($($development:ident),* $(,)?) => {$(
		impl SiteDevelopment for $development {
			const KIND: DevelopmentKind = DevelopmentKind::$development;

			fn into_built(self) -> BuiltDevelopment {
				BuiltDevelopment::$development(Box::new(self))
			}
		}
	)*};
}

site_developments!(
	LesHalles,
	ShepherdsVillage,
	ShepherdsCommune,
	RingFort,
	TempleComplex,
	SingleHighrise,
	SuburbanHomes,
	WizardsTower,
	SkybridgeBazaar,
	OldCityMarket,
);

/// Development `D` planned on one site over ground `G`.
pub struct DevelopmentCell<D: Development, G> {
	pub cell: Aabb3d,
	pub plan: D::Plan,
	pub pads: Vec<DevelopmentPad>,
	_ground: PhantomData<fn() -> G>,
}

impl<D: SiteDevelopment, G> DevelopmentCell<D, G> {
	pub fn built(&self) -> Option<BuiltDevelopment> {
		D::build(&self.plan).map(D::into_built)
	}
}

impl<D: SiteDevelopment, G: RichmondGround> GenerationScheme<HcsgStorage>
	for DevelopmentCell<D, G>
{
	fn original_ids_for(storage: &mut HcsgStorage, region: Aabb3d) -> Vec<OriginalId> {
		DevelopmentSite::ids_of_kind(storage, region, D::KIND)
	}

	/// An authored site plans on its own level, dry ground; a procedural one
	/// over `G`.
	fn build_with_id(storage: &mut HcsgStorage, id: Id) -> Option<(Self, Aabb3d)> {
		let (site, config) = DevelopmentSite::planned(storage, id, D::KIND)?;
		let bounds = column_bounds(site.cell);
		let (plan, pads) = match site.authored {
			Some(mut authored) => D::plan(&mut authored, site.cell, config.seed)?,
			None => {
				let mut ground = GroundSampler::<G>::new(storage, bounds);
				D::plan(&mut ground, site.cell, config.seed)?
			}
		};
		let pads = pads.iter().map(DevelopmentPad::from).collect();
		Some((Self { cell: site.cell, plan, pads, _ground: PhantomData }, bounds))
	}
}

/// One development cell over ground `G`.
pub enum RichmondDevelopment<G> {
	Empty(Aabb3d),
	LesHalles(DevelopmentCell<LesHalles, G>),
	ShepherdsVillage(DevelopmentCell<ShepherdsVillage, G>),
	ShepherdsCommune(DevelopmentCell<ShepherdsCommune, G>),
	RingFort(DevelopmentCell<RingFort, G>),
	TempleComplex(DevelopmentCell<TempleComplex, G>),
	SingleHighrise(DevelopmentCell<SingleHighrise, G>),
	SuburbanHomes(DevelopmentCell<SuburbanHomes, G>),
	WizardsTower(DevelopmentCell<WizardsTower, G>),
	SkybridgeBazaar(DevelopmentCell<SkybridgeBazaar, G>),
	OldCityMarket(DevelopmentCell<OldCityMarket, G>),
}

impl<G: RichmondGround> GenerationScheme<HcsgStorage> for RichmondDevelopment<G> {
	fn original_ids_for(storage: &mut HcsgStorage, region: Aabb3d) -> Vec<OriginalId> {
		DevelopmentSite::original_ids_for(storage, region)
	}

	/// A site whose kinds all fail to fit is stored [`Self::Empty`]. A
	/// procedural site without ground under its center has no answer yet.
	fn build_with_id(storage: &mut HcsgStorage, id: Id) -> Option<(Self, Aabb3d)> {
		let site = storage.get_one_or_generate::<DevelopmentSite>(id)?.clone();
		let bounds = column_bounds(site.cell);
		if !site.is_filled() {
			return Some((Self::Empty(site.cell), bounds));
		}
		if site.authored.is_none() {
			let center = cell_center_xz(site.cell);
			GroundSampler::<G>::new(storage, bounds).height_at(center.x, center.y)?;
		}
		for kind in site.kinds() {
			let Some(development) = Self::build_kind(storage, id, kind) else {
				continue;
			};
			let Some(authored) = &site.authored else {
				return Some((development, bounds));
			};
			let development = match authored.courtyard {
				Some(courtyard) => development.with_authored_courtyard(courtyard),
				None => Some(development),
			};
			if let Some(development) = development.filter(|d| d.built().is_some()) {
				return Some((development, bounds));
			}
		}
		Some((Self::Empty(site.cell), bounds))
	}
}

impl<G: RichmondGround> RichmondDevelopment<G> {
	/// Pad nodes of stored filled developments affecting `region`, merged
	/// into one sample-time blend pass.
	///
	/// One complex matters for overlapping pads: sequential modulation
	/// would let later ease skirts smear earlier exact terraces.
	pub fn merged_pads(storage: &HcsgStorage, region: Aabb3d) -> PadComplex {
		let bounds = Bounds2::from_xz(region.min.x, region.min.z, region.max.x, region.max.z);
		let nodes = storage
			.overlapping::<Self>(column_bounds(region))
			.into_iter()
			.filter_map(|id| storage.get::<Self>(id))
			.filter(|development| {
				development.is_filled() && overlaps_xz_strictly(region, development.cell())
			})
			.flat_map(Self::pad_complexes)
			.flat_map(|complex| complex.pads.iter())
			.filter(|node| node.correction_intersects(bounds))
			.cloned()
			.collect();
		PadComplex::from_nodes(nodes)
	}

	fn build_kind(storage: &mut HcsgStorage, id: Id, kind: DevelopmentKind) -> Option<Self> {
		fn cell<D: SiteDevelopment, G: RichmondGround>(
			storage: &mut HcsgStorage,
			id: Id,
		) -> Option<DevelopmentCell<D, G>> {
			DevelopmentCell::<D, G>::build_with_id(storage, id).map(|(cell, _)| cell)
		}
		Some(match kind {
			DevelopmentKind::Empty => return None,
			DevelopmentKind::LesHalles => Self::LesHalles(cell(storage, id)?),
			DevelopmentKind::ShepherdsVillage => Self::ShepherdsVillage(cell(storage, id)?),
			DevelopmentKind::ShepherdsCommune => Self::ShepherdsCommune(cell(storage, id)?),
			DevelopmentKind::RingFort => Self::RingFort(cell(storage, id)?),
			DevelopmentKind::TempleComplex => Self::TempleComplex(cell(storage, id)?),
			DevelopmentKind::SingleHighrise => Self::SingleHighrise(cell(storage, id)?),
			DevelopmentKind::SuburbanHomes => Self::SuburbanHomes(cell(storage, id)?),
			DevelopmentKind::WizardsTower => Self::WizardsTower(cell(storage, id)?),
			DevelopmentKind::SkybridgeBazaar => Self::SkybridgeBazaar(cell(storage, id)?),
			DevelopmentKind::OldCityMarket => Self::OldCityMarket(cell(storage, id)?),
		})
	}

	/// Re-pads a single-terrace kind as the authored walled courtyard.
	fn with_authored_courtyard(self, courtyard: AuthoredCourtyard) -> Option<Self> {
		let footprint = self.footprint_half_extents()?;
		let half = (footprint + Vec2::splat(courtyard.margin)).min(Vec2::splat(courtyard.max_half))
			+ Vec2::splat(courtyard.overhang);
		self.with_courtyard(half, PadParams { berm: 0.0, ease: courtyard.ease, round: 0.0 })
	}
}

impl<G> RichmondDevelopment<G> {
	pub fn cell(&self) -> Aabb3d {
		match self {
			Self::Empty(cell) => *cell,
			Self::LesHalles(d) => d.cell,
			Self::ShepherdsVillage(d) => d.cell,
			Self::ShepherdsCommune(d) => d.cell,
			Self::RingFort(d) => d.cell,
			Self::TempleComplex(d) => d.cell,
			Self::SingleHighrise(d) => d.cell,
			Self::SuburbanHomes(d) => d.cell,
			Self::WizardsTower(d) => d.cell,
			Self::SkybridgeBazaar(d) => d.cell,
			Self::OldCityMarket(d) => d.cell,
		}
	}

	pub fn kind(&self) -> DevelopmentKind {
		match self {
			Self::Empty(_) => DevelopmentKind::Empty,
			Self::LesHalles(_) => DevelopmentKind::LesHalles,
			Self::ShepherdsVillage(_) => DevelopmentKind::ShepherdsVillage,
			Self::ShepherdsCommune(_) => DevelopmentKind::ShepherdsCommune,
			Self::RingFort(_) => DevelopmentKind::RingFort,
			Self::TempleComplex(_) => DevelopmentKind::TempleComplex,
			Self::SingleHighrise(_) => DevelopmentKind::SingleHighrise,
			Self::SuburbanHomes(_) => DevelopmentKind::SuburbanHomes,
			Self::WizardsTower(_) => DevelopmentKind::WizardsTower,
			Self::SkybridgeBazaar(_) => DevelopmentKind::SkybridgeBazaar,
			Self::OldCityMarket(_) => DevelopmentKind::OldCityMarket,
		}
	}

	pub fn is_filled(&self) -> bool {
		!matches!(self, Self::Empty(_))
	}

	/// The terrace, for kinds fitted to one terrace.
	pub fn terrace(&self) -> Option<&Terrace> {
		match self {
			Self::LesHalles(d) => Some(&d.plan),
			Self::RingFort(d) => Some(&d.plan),
			Self::TempleComplex(d) => Some(&d.plan),
			Self::SingleHighrise(d) => Some(&d.plan),
			Self::SuburbanHomes(d) => Some(&d.plan),
			Self::WizardsTower(d) => Some(&d.plan),
			Self::SkybridgeBazaar(d) => Some(&d.plan),
			Self::Empty(_)
			| Self::ShepherdsVillage(_)
			| Self::ShepherdsCommune(_)
			| Self::OldCityMarket(_) => None,
		}
	}

	pub fn pads(&self) -> &[DevelopmentPad] {
		match self {
			Self::Empty(_) => &[],
			Self::LesHalles(d) => &d.pads,
			Self::ShepherdsVillage(d) => &d.pads,
			Self::ShepherdsCommune(d) => &d.pads,
			Self::RingFort(d) => &d.pads,
			Self::TempleComplex(d) => &d.pads,
			Self::SingleHighrise(d) => &d.pads,
			Self::SuburbanHomes(d) => &d.pads,
			Self::WizardsTower(d) => &d.pads,
			Self::SkybridgeBazaar(d) => &d.pads,
			Self::OldCityMarket(d) => &d.pads,
		}
	}

	fn terrace_pads_mut(&mut self) -> Option<(&Terrace, &mut Vec<DevelopmentPad>)> {
		match self {
			Self::LesHalles(d) => Some((&d.plan, &mut d.pads)),
			Self::RingFort(d) => Some((&d.plan, &mut d.pads)),
			Self::TempleComplex(d) => Some((&d.plan, &mut d.pads)),
			Self::SingleHighrise(d) => Some((&d.plan, &mut d.pads)),
			Self::SuburbanHomes(d) => Some((&d.plan, &mut d.pads)),
			Self::WizardsTower(d) => Some((&d.plan, &mut d.pads)),
			Self::SkybridgeBazaar(d) => Some((&d.plan, &mut d.pads)),
			Self::Empty(_)
			| Self::ShepherdsVillage(_)
			| Self::ShepherdsCommune(_)
			| Self::OldCityMarket(_) => None,
		}
	}

	pub fn pad_complexes(&self) -> impl Iterator<Item = &PadComplex> {
		self.pads().iter().map(|pad| &pad.complex)
	}

	/// World-axis half extents of the yawed building confines.
	pub fn footprint_half_extents(&self) -> Option<Vec2> {
		self.terrace().map(Terrace::footprint_half_extents)
	}

	/// Replace a single-terrace pad with one axis-aligned terrace at the same
	/// height. `None` for kinds whose pads sit at several heights.
	pub fn with_courtyard(mut self, half_extents: Vec2, params: PadParams) -> Option<Self> {
		let (terrace, pads) = self.terrace_pads_mut()?;
		let courtyard =
			PadPlan::building_skirt(terrace.center_xz(), half_extents, 0.0, terrace.height, params);
		*pads = vec![DevelopmentPad::from(&courtyard)];
		Some(self)
	}

	/// The buildings fitted to a filled cell.
	pub fn built(&self) -> Option<BuiltDevelopment> {
		match self {
			Self::Empty(_) => None,
			Self::LesHalles(d) => d.built(),
			Self::ShepherdsVillage(d) => d.built(),
			Self::ShepherdsCommune(d) => d.built(),
			Self::RingFort(d) => d.built(),
			Self::TempleComplex(d) => d.built(),
			Self::SingleHighrise(d) => d.built(),
			Self::SuburbanHomes(d) => d.built(),
			Self::WizardsTower(d) => d.built(),
			Self::SkybridgeBazaar(d) => d.built(),
			Self::OldCityMarket(d) => d.built(),
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::cell::DevelopmentExtent;
	use crate::config::DevelopmentConfig;

	fn les_halles() -> RichmondDevelopment<()> {
		let cell = DevelopmentExtent::from_cell_index(0, 0).aabb();
		let terrace = Terrace::new::<LesHalles>(cell, 12.0, DevelopmentConfig::default().seed);
		let pads = vec![DevelopmentPad::from(&terrace.pad())];
		RichmondDevelopment::LesHalles(DevelopmentCell {
			cell,
			plan: terrace,
			pads,
			_ground: PhantomData,
		})
	}

	#[test]
	fn terrace_pad_flattens_the_building_center() {
		let development = les_halles();
		let c = cell_center_xz(development.cell());
		let pad = &development.pads()[0].complex;
		assert!((pad.modify_elevation(3.0, c.x, c.y) - 12.0).abs() < 1e-3);
		assert!((pad.modify_elevation(3.0, 400.0, 400.0) - 3.0).abs() < 1e-3);
	}

	#[test]
	fn courtyard_flattens_the_whole_arena_at_the_pad_height() -> anyhow::Result<()> {
		let development = les_halles();
		let half = development
			.footprint_half_extents()
			.ok_or_else(|| anyhow::anyhow!("terrace footprint"))?
			+ Vec2::splat(20.0);
		let walled = development
			.with_courtyard(half, PadParams { berm: 0.0, ease: 16.0, round: 0.0 })
			.ok_or_else(|| anyhow::anyhow!("terrace courtyard"))?;
		let center = cell_center_xz(walled.cell());
		anyhow::ensure!(walled.pads().len() == 1);
		let pad = &walled.pads()[0].complex;
		for corner in [Vec2::new(1.0, 1.0), Vec2::new(-1.0, 1.0), Vec2::new(1.0, -1.0)] {
			let p = center + corner * (half - Vec2::splat(0.5));
			let y = pad.modify_elevation(-30.0, p.x, p.y);
			anyhow::ensure!((y - 12.0).abs() <= 1e-3, "corner {p} at {y}, expected 12");
		}
		Ok(())
	}
}
