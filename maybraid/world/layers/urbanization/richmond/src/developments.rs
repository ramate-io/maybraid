//! [`RichmondDevelopment`]: the development a [`DevelopmentSite`] gets.
//!
//! Each kind is its own [`GenerationScheme`] in a submodule; the enum
//! generates the site, then the first of its kinds that fits.

pub mod les_halles;
pub mod old_city_market;
pub mod ring_fort;
pub mod shepherds_commune;
pub mod shepherds_village;
pub mod single_highrise;
pub mod site;
pub mod skybridge_bazaar;
pub mod solitary;
pub mod suburban_homes;
pub mod temple_complex;
pub mod wizards_tower;

use bevy::math::bounding::Aabb3d;
use bevy::math::Vec2;
use lod::gen::{GenerationScheme, Id, OriginalId};
use lod::hcsg::HcsgStorage;
use procedural_common::NoiseParams;

use crate::artifact::BuiltDevelopment;
use crate::ground::{GroundSampler, RichmondGround, SiteGround};
use crate::pad::{cell_center_xz, PadComplex, PadParams};
use crate::storage::column_bounds;

use les_halles::LesHallesCell;
use old_city_market::OldCityMarketCell;
use ring_fort::RingFortCell;
use shepherds_commune::ShepherdsCommuneCell;
use shepherds_village::ShepherdsVillageCell;
use single_highrise::SingleHighriseCell;
use site::{AuthoredCourtyard, DevelopmentKind, DevelopmentSite};
use skybridge_bazaar::SkybridgeBazaarCell;
use solitary::SolitaryPlan;
use suburban_homes::SuburbanHomesCell;
use temple_complex::TempleComplexCell;
use wizards_tower::WizardsTowerCell;

/// Pad baked from a post-Watershed height sample: flatten terrace + ease skirt.
#[derive(Debug, Clone)]
pub struct DevelopmentPad {
	pub height: f32,
	pub complex: PadComplex,
}

/// One development cell over ground `G`.
pub enum RichmondDevelopment<G> {
	Empty(Aabb3d),
	LesHalles(LesHallesCell<G>),
	ShepherdsVillage(ShepherdsVillageCell<G>),
	ShepherdsCommune(ShepherdsCommuneCell<G>),
	RingFort(RingFortCell<G>),
	TempleComplex(TempleComplexCell<G>),
	SingleHighrise(SingleHighriseCell<G>),
	SuburbanHomes(SuburbanHomesCell<G>),
	WizardsTower(WizardsTowerCell<G>),
	SkybridgeBazaar(SkybridgeBazaarCell<G>),
	OldCityMarket(OldCityMarketCell<G>),
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
			let seed = authored.config.seed as i32;
			let development = match authored.courtyard {
				Some(courtyard) => development.with_authored_courtyard(courtyard),
				None => Some(development),
			};
			if let Some(development) = development.filter(|d| d.built(seed).is_some()) {
				return Some((development, bounds));
			}
		}
		Some((Self::Empty(site.cell), bounds))
	}
}

impl<G: RichmondGround> RichmondDevelopment<G> {
	fn build_kind(storage: &mut HcsgStorage, id: Id, kind: DevelopmentKind) -> Option<Self> {
		Some(match kind {
			DevelopmentKind::Empty => return None,
			DevelopmentKind::LesHalles => {
				Self::LesHalles(LesHallesCell::<G>::build_with_id(storage, id)?.0)
			}
			DevelopmentKind::ShepherdsVillage => {
				Self::ShepherdsVillage(ShepherdsVillageCell::<G>::build_with_id(storage, id)?.0)
			}
			DevelopmentKind::ShepherdsCommune => {
				Self::ShepherdsCommune(ShepherdsCommuneCell::<G>::build_with_id(storage, id)?.0)
			}
			DevelopmentKind::RingFort => {
				Self::RingFort(RingFortCell::<G>::build_with_id(storage, id)?.0)
			}
			DevelopmentKind::TempleComplex => {
				Self::TempleComplex(TempleComplexCell::<G>::build_with_id(storage, id)?.0)
			}
			DevelopmentKind::SingleHighrise => {
				Self::SingleHighrise(SingleHighriseCell::<G>::build_with_id(storage, id)?.0)
			}
			DevelopmentKind::SuburbanHomes => {
				Self::SuburbanHomes(SuburbanHomesCell::<G>::build_with_id(storage, id)?.0)
			}
			DevelopmentKind::WizardsTower => {
				Self::WizardsTower(WizardsTowerCell::<G>::build_with_id(storage, id)?.0)
			}
			DevelopmentKind::SkybridgeBazaar => {
				Self::SkybridgeBazaar(SkybridgeBazaarCell::<G>::build_with_id(storage, id)?.0)
			}
			DevelopmentKind::OldCityMarket => {
				Self::OldCityMarket(OldCityMarketCell::<G>::build_with_id(storage, id)?.0)
			}
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
			Self::ShepherdsVillage(village) => village.cell,
			Self::ShepherdsCommune(commune) => commune.cell,
			Self::OldCityMarket(market) => market.cell,
			Self::LesHalles(cell) => cell.plan.cell,
			Self::RingFort(cell) => cell.plan.cell,
			Self::TempleComplex(cell) => cell.plan.cell,
			Self::SingleHighrise(cell) => cell.plan.cell,
			Self::SuburbanHomes(cell) => cell.plan.cell,
			Self::WizardsTower(cell) => cell.plan.cell,
			Self::SkybridgeBazaar(cell) => cell.plan.cell,
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

	/// The single-terrace plan, for kinds that build one.
	pub fn solitary(&self) -> Option<&SolitaryPlan> {
		match self {
			Self::LesHalles(cell) => Some(&cell.plan),
			Self::RingFort(cell) => Some(&cell.plan),
			Self::TempleComplex(cell) => Some(&cell.plan),
			Self::SingleHighrise(cell) => Some(&cell.plan),
			Self::SuburbanHomes(cell) => Some(&cell.plan),
			Self::WizardsTower(cell) => Some(&cell.plan),
			Self::SkybridgeBazaar(cell) => Some(&cell.plan),
			Self::Empty(_)
			| Self::ShepherdsVillage(_)
			| Self::ShepherdsCommune(_)
			| Self::OldCityMarket(_) => None,
		}
	}

	fn solitary_mut(&mut self) -> Option<&mut SolitaryPlan> {
		match self {
			Self::LesHalles(cell) => Some(&mut cell.plan),
			Self::RingFort(cell) => Some(&mut cell.plan),
			Self::TempleComplex(cell) => Some(&mut cell.plan),
			Self::SingleHighrise(cell) => Some(&mut cell.plan),
			Self::SuburbanHomes(cell) => Some(&mut cell.plan),
			Self::WizardsTower(cell) => Some(&mut cell.plan),
			Self::SkybridgeBazaar(cell) => Some(&mut cell.plan),
			Self::Empty(_)
			| Self::ShepherdsVillage(_)
			| Self::ShepherdsCommune(_)
			| Self::OldCityMarket(_) => None,
		}
	}

	pub fn pads(&self) -> &[DevelopmentPad] {
		match self {
			Self::Empty(_) => &[],
			Self::ShepherdsVillage(village) => &village.pads,
			Self::ShepherdsCommune(commune) => &commune.pads,
			Self::OldCityMarket(market) => &market.pads,
			Self::LesHalles(cell) => std::slice::from_ref(&cell.plan.pad),
			Self::RingFort(cell) => std::slice::from_ref(&cell.plan.pad),
			Self::TempleComplex(cell) => std::slice::from_ref(&cell.plan.pad),
			Self::SingleHighrise(cell) => std::slice::from_ref(&cell.plan.pad),
			Self::SuburbanHomes(cell) => std::slice::from_ref(&cell.plan.pad),
			Self::WizardsTower(cell) => std::slice::from_ref(&cell.plan.pad),
			Self::SkybridgeBazaar(cell) => std::slice::from_ref(&cell.plan.pad),
		}
	}

	pub fn pad_complexes(&self) -> impl Iterator<Item = &PadComplex> {
		self.pads().iter().map(|pad| &pad.complex)
	}

	/// World-axis half extents of the yawed building confines.
	pub fn footprint_half_extents(&self) -> Option<Vec2> {
		self.solitary().map(SolitaryPlan::footprint_half_extents)
	}

	/// Replace a single-terrace pad with one axis-aligned terrace at the same
	/// height. `None` for kinds whose pads sit at several heights.
	pub fn with_courtyard(mut self, half_extents: Vec2, params: PadParams) -> Option<Self> {
		self.solitary_mut()?.flatten_courtyard(half_extents, params);
		Some(self)
	}

	/// Fit hosts for a filled cell. `seed` is the Richmond noise seed.
	pub fn built(&self, seed: i32) -> Option<BuiltDevelopment> {
		let noise = NoiseParams { seed, ..NoiseParams::default() };
		match self {
			Self::Empty(_) => None,
			Self::LesHalles(cell) => cell.built(noise),
			Self::RingFort(cell) => cell.built(noise),
			Self::TempleComplex(cell) => cell.built(noise),
			Self::SingleHighrise(cell) => cell.built(noise),
			Self::SuburbanHomes(cell) => cell.built(noise),
			Self::WizardsTower(cell) => cell.built(noise),
			Self::SkybridgeBazaar(cell) => cell.built(noise),
			Self::ShepherdsVillage(village) => Some(village.built()),
			Self::ShepherdsCommune(commune) => Some(commune.built()),
			Self::OldCityMarket(market) => Some(market.built()),
		}
	}
}
