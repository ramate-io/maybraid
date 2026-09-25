//! Stamp one filled development without urbanization occupancy.

use bevy::math::Vec2;
use bevy::math::bounding::Aabb3d;
use durham_terrain_models::{TerrainCellLayout, TerrainEntryStore};
use procedural_common::{NoiseParams, SeededHash};
use richmond_buildings::Fit;
use richmond_developments::PlacedBuilding;

use crate::archetype_generation::ArchetypeGenerator;
use crate::artifact::BuiltDevelopment;
use crate::commune::build_shepherds_commune;
use crate::config::DevelopmentConfig;
use crate::development::{DevelopmentCell, DevelopmentContent, DevelopmentKind, cell_salt};
use crate::les_halles::LesHallesDevelopment;
use crate::pad::{PadComplex, PadParams};
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

	/// World-axis half extents of the yawed building confines.
	pub fn footprint_half_extents(&self) -> Option<Vec2> {
		let (extent, yaw) = match &self.content {
			DevelopmentContent::LesHalles(content) => {
				(content.confines_extent_xz, content.confines_yaw)
			}
			DevelopmentContent::RingFort(content) => {
				(content.confines_extent_xz, content.confines_yaw)
			}
			DevelopmentContent::Archetype(content) => {
				(content.confines_extent_xz, content.confines_yaw)
			}
			_ => return None,
		};
		let half = extent * 0.5;
		let (sin, cos) = yaw.sin_cos();
		let (sin, cos) = (sin.abs(), cos.abs());
		Some(Vec2::new(cos * half.x + sin * half.y, sin * half.x + cos * half.y))
	}

	/// Replace a single-terrace pad with one axis-aligned terrace at the same
	/// height. `None` for kinds whose pads sit at several heights.
	pub fn with_courtyard(mut self, half_extents: Vec2, params: PadParams) -> Option<Self> {
		let center = crate::pad::cell_center_xz(self.cell);
		let pad = match &mut self.content {
			DevelopmentContent::LesHalles(content) => &mut content.pad,
			DevelopmentContent::RingFort(content) => &mut content.pad,
			DevelopmentContent::Archetype(content) => &mut content.pad,
			_ => return None,
		};
		pad.complex = PadComplex::building_skirt(center, half_extents, 0.0, pad.height, params);
		Some(self)
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
	fn courtyard_flattens_the_whole_arena_at_the_pad_height() -> Result<(), String> {
		let cell = DevelopmentExtent::from_cell_index(0, 0).aabb();
		let config = DevelopmentConfig { seed: 42, ..DevelopmentConfig::default() };
		let filled = DevelopmentCell::with_les_halles(cell, 12.0, &config);
		let footprint = filled.footprint_half_extents().ok_or("footprint")?;
		let half = footprint + Vec2::splat(20.0);
		let params = PadParams { berm: 0.0, ease: 16.0, round: 0.0 };
		let walled = filled.with_courtyard(half, params).ok_or("courtyard")?;
		let center = crate::pad::cell_center_xz(cell);
		let complex = walled.pad_complexes().next().ok_or("pad")?;
		for corner in [Vec2::new(1.0, 1.0), Vec2::new(-1.0, 1.0), Vec2::new(1.0, -1.0)] {
			let p = center + corner * (half - Vec2::splat(0.5));
			let y = complex.modify_elevation(-30.0, p.x, p.y);
			if (y - 12.0).abs() > 1e-3 {
				return Err(format!("corner {p} at {y}, expected 12"));
			}
		}
		Ok(())
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
