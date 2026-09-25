//! Stamp one filled development without urbanization occupancy.

use bevy::math::bounding::Aabb3d;
use durham_terrain_models::{TerrainCellLayout, TerrainEntryStore};
use procedural_common::{NoiseParams, SeededHash};
use richmond_buildings::Fit;
use richmond_developments::PlacedBuilding;

use crate::archetype_generation::ArchetypeGenerator;
use crate::artifact::BuiltDevelopment;
use crate::commune::build_shepherds_commune;
use crate::config::DevelopmentConfig;
use crate::development::{DevelopmentCell, DevelopmentKind, cell_salt};
use crate::les_halles::LesHallesDevelopment;
use crate::ring_fort::RingFortDevelopment;
use crate::shepherds::{ShepherdsCommuneDevelopment, ShepherdsVillageDevelopment};
use crate::village::build_shepherds_village;

impl DevelopmentKind {
	/// Every authored fill. Occupancy may still pick [`Self::Empty`].
	pub const FILLED: [Self; 10] = [
		Self::LesHalles,
		Self::ShepherdsVillage,
		Self::ShepherdsCommune,
		Self::RingFort,
		Self::TempleComplex,
		Self::SingleHighrise,
		Self::SuburbanHomes,
		Self::WizardsTower,
		Self::SkybridgeBazaar,
		Self::OldCityMarket,
	];

	/// Weighted pick that always returns a building, never [`Self::Empty`].
	pub fn pick_filled(cell: Aabb3d, config: &DevelopmentConfig) -> Self {
		let weighted = [
			(Self::LesHalles, config.les_halles_weight),
			(Self::ShepherdsVillage, config.shepherds_village_weight),
			(Self::ShepherdsCommune, config.shepherds_commune_weight),
			(Self::RingFort, config.ring_fort_weight),
			(Self::TempleComplex, config.temple_complex_weight),
			(Self::SingleHighrise, config.single_highrise_weight),
			(Self::SuburbanHomes, config.suburban_homes_weight),
			(Self::WizardsTower, config.wizards_tower_weight),
			(Self::SkybridgeBazaar, config.skybridge_bazaar_weight),
			(Self::OldCityMarket, config.old_city_market_weight),
		];
		let total: f32 = weighted.iter().map(|(_, weight)| weight.max(0.0)).sum();
		if total <= f32::EPSILON {
			return Self::SingleHighrise;
		}
		let hash = SeededHash::new(config.seed.wrapping_add(cell_salt(cell)));
		let mut pick = hash.unit(44) * total;
		for (kind, weight) in weighted {
			let weight = weight.max(0.0);
			if weight > 0.0 && pick < weight {
				return kind;
			}
			pick -= weight;
		}
		Self::OldCityMarket
	}
}

impl DevelopmentCell {
	/// Author one filled cell of `kind` on `height`. Village / market builders
	/// sample the store; pad-only kinds use `height` as the terrace.
	pub fn fill(
		store: &TerrainEntryStore,
		layout: &TerrainCellLayout,
		cell: Aabb3d,
		kind: DevelopmentKind,
		config: &DevelopmentConfig,
		height: f32,
	) -> Option<Self> {
		Some(match kind {
			DevelopmentKind::Empty => return None,
			DevelopmentKind::LesHalles => Self::with_les_halles(cell, height, config),
			DevelopmentKind::ShepherdsVillage => {
				let (village, pads) = build_shepherds_village(store, layout, cell, config)?;
				Self::with_shepherds_village(cell, village, pads)
			}
			DevelopmentKind::ShepherdsCommune => {
				let (commune, pads) = build_shepherds_commune(store, layout, cell, config)?;
				Self::with_shepherds_commune(cell, commune, pads)
			}
			DevelopmentKind::OldCityMarket => {
				let (market, pads) =
					ArchetypeGenerator::build_old_city_market(store, layout, cell, config)?;
				Self::with_old_city_market(cell, market, pads)
			}
			DevelopmentKind::RingFort => Self::with_ring_fort(cell, height, config),
			kind @ (DevelopmentKind::TempleComplex
			| DevelopmentKind::SingleHighrise
			| DevelopmentKind::SuburbanHomes
			| DevelopmentKind::WizardsTower
			| DevelopmentKind::SkybridgeBazaar) => Self::with_archetype(cell, height, kind, config),
		})
	}

	/// Fit hosts for a filled cell. `seed` is the Richmond noise seed.
	pub fn built(&self, seed: i32) -> Option<BuiltDevelopment> {
		let cell_aabb = self.cell;
		let noise = NoiseParams { seed, ..NoiseParams::default() };
		Some(match self.kind() {
			DevelopmentKind::Empty => return None,
			DevelopmentKind::LesHalles => {
				let content = self.les_halles()?;
				let finish = content.finish.clone();
				let (development, _) =
					richmond_developments::MixedUseLesHallesDevelopment::fit_to_confines(
						&self.confines()?,
						noise,
					)
					.ok()?;
				BuiltDevelopment::LesHalles(Box::new(LesHallesDevelopment {
					cell: cell_aabb,
					building: PlacedBuilding {
						center_xz: crate::pad::cell_center_xz(cell_aabb),
						yaw: content.confines_yaw,
						footprint: content.confines_extent_xz,
						ground_height: content.pad.height,
						building: development.with_finish(finish.wall, finish.roof),
					},
				}))
			}
			DevelopmentKind::ShepherdsVillage => {
				BuiltDevelopment::ShepherdsVillage(Box::new(ShepherdsVillageDevelopment {
					village: self.shepherds_village()?.village.clone(),
				}))
			}
			DevelopmentKind::ShepherdsCommune => {
				BuiltDevelopment::ShepherdsCommune(Box::new(ShepherdsCommuneDevelopment {
					commune: self.shepherds_commune()?.commune.clone(),
				}))
			}
			DevelopmentKind::RingFort => {
				let content = self.ring_fort()?;
				let finish = content.finish.clone();
				let (development, _) =
					richmond_developments::RingFort::fit_to_confines(&self.confines()?, noise)
						.ok()?;
				BuiltDevelopment::RingFort(Box::new(RingFortDevelopment {
					cell: cell_aabb,
					building: PlacedBuilding {
						center_xz: crate::pad::cell_center_xz(cell_aabb),
						yaw: content.confines_yaw,
						footprint: content.confines_extent_xz,
						ground_height: content.pad.height,
						building: development.with_finish(finish.wall, finish.roof),
					},
				}))
			}
			DevelopmentKind::TempleComplex => {
				let finish = self.archetype()?.finish.clone();
				BuiltDevelopment::TempleComplex(Box::new(
					ArchetypeGenerator::build_temple_complex(cell_aabb, &self.confines()?, noise)?
						.with_finish(finish.wall, finish.roof),
				))
			}
			DevelopmentKind::SingleHighrise => {
				let wall = self.archetype()?.finish.wall.clone();
				let mut development =
					ArchetypeGenerator::build_single_highrise(cell_aabb, self.confines()?, noise)?;
				development.building.building =
					development.building.building.with_wall_material(wall);
				BuiltDevelopment::SingleHighrise(Box::new(development))
			}
			DevelopmentKind::SuburbanHomes => BuiltDevelopment::SuburbanHomes(Box::new(
				ArchetypeGenerator::build_suburban_homes(cell_aabb, &self.confines()?, noise)?,
			)),
			DevelopmentKind::WizardsTower => {
				let finish = self.archetype()?.finish.clone();
				let mut development =
					ArchetypeGenerator::build_wizards_tower(cell_aabb, self.confines()?, noise)?;
				development.building.building =
					development.building.building.with_finish(finish.wall, finish.roof);
				BuiltDevelopment::WizardsTower(Box::new(development))
			}
			DevelopmentKind::SkybridgeBazaar => {
				let connector = self.archetype()?.finish.wall.clone();
				BuiltDevelopment::SkybridgeBazaar(Box::new(
					ArchetypeGenerator::build_skybridge_bazaar(
						cell_aabb,
						&self.confines()?,
						noise,
					)?
					.with_bridge_material(connector),
				))
			}
			DevelopmentKind::OldCityMarket => {
				BuiltDevelopment::OldCityMarket(Box::new(self.old_city_market()?.market.clone()))
			}
		})
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::cell::DevelopmentExtent;

	#[test]
	fn pick_filled_never_returns_empty() {
		let cell = DevelopmentExtent::from_cell_index(0, 0).aabb();
		for seed in 0..48u32 {
			let config = DevelopmentConfig { seed, ..DevelopmentConfig::default() };
			assert_ne!(DevelopmentKind::pick_filled(cell, &config), DevelopmentKind::Empty);
		}
	}

	#[test]
	fn pick_filled_is_seeded_and_not_only_les_halles() {
		let cell = DevelopmentExtent::from_cell_index(0, 0).aabb();
		let a = DevelopmentKind::pick_filled(
			cell,
			&DevelopmentConfig { seed: 42, ..Default::default() },
		);
		let b = DevelopmentKind::pick_filled(
			cell,
			&DevelopmentConfig { seed: 42, ..Default::default() },
		);
		assert_eq!(a, b);
		let kinds: Vec<_> = (0..32u32)
			.map(|seed| {
				DevelopmentKind::pick_filled(
					cell,
					&DevelopmentConfig { seed, ..DevelopmentConfig::default() },
				)
			})
			.collect();
		assert!(
			kinds.iter().any(|kind| *kind != DevelopmentKind::LesHalles),
			"expected a non-Les-Halles pick in 32 seeds, got {kinds:?}"
		);
	}
}
