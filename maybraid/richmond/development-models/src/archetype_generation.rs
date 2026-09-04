//! Builders for the shared solitary, campus, and neighborhood layout families.

use bevy::math::bounding::Aabb3d;
use bevy::math::{Vec2, Vec3};
use bevy::transform::components::Transform;
use procedural_common::{Bounds2, NoiseParams, SeededHash};
use richmond_buildings::{Confines, Fit, Openings};
use richmond_developments::{
	OldCityMarket, PlacedBuilding, SingleHighrise, Skybridge, SkybridgeBazaar,
	SolitaryWizardsTower, SuburbanHomes, TempleComplex,
};

use crate::cell::yaw_about_xz;
use crate::scatter::{bounds_intersect, ScatterChoice, ScatterRecipe};
use crate::shepherds_fit::{fit_shepherds_building, ShepherdsBuildingKind};

/// One solitary fitted building and the cell that owns it.
#[derive(Debug, Clone)]
pub struct PlacedDevelopment<T> {
	pub cell: Aabb3d,
	pub building: PlacedBuilding<T>,
}

impl<T> PlacedDevelopment<T> {
	pub fn host_transform(&self) -> Transform {
		yaw_about_xz(self.building.center_xz, self.building.yaw)
	}
}

/// Constructs the catalog's shared layout families from a selected envelope.
pub(crate) struct ArchetypeGenerator;

impl ArchetypeGenerator {
	pub(crate) fn build_single_highrise(
		cell: Aabb3d,
		confines: Confines,
		noise: NoiseParams,
	) -> Option<PlacedDevelopment<SingleHighrise>> {
		let center = confines.center_xz();
		let yaw = confines.roll;
		let footprint = confines.footprint();
		let ground_height = confines.bounds.min.y;
		let (building, _) = SingleHighrise::fit_to_confines(&confines, noise).ok()?;
		Some(PlacedDevelopment {
			cell,
			building: PlacedBuilding { center_xz: center, yaw, footprint, ground_height, building },
		})
	}

	pub(crate) fn build_wizards_tower(
		cell: Aabb3d,
		confines: Confines,
		noise: NoiseParams,
	) -> Option<PlacedDevelopment<SolitaryWizardsTower>> {
		let center = confines.center_xz();
		let yaw = confines.roll;
		let footprint = confines.footprint();
		let ground_height = confines.bounds.min.y;
		let (building, _) = SolitaryWizardsTower::fit_to_confines(&confines, noise).ok()?;
		Some(PlacedDevelopment {
			cell,
			building: PlacedBuilding { center_xz: center, yaw, footprint, ground_height, building },
		})
	}

	pub(crate) fn build_suburban_homes(
		cell: Aabb3d,
		confines: &Confines,
		noise: NoiseParams,
	) -> Option<SuburbanHomes> {
		let recipe = ScatterRecipe {
			grid_side: 4,
			min_count: 7,
			max_count: 11,
			cell_inset: (cell.max.x - confines.bounds.max.x).abs() + 24.0,
			jitter: 9.0,
			clearance: 8.0,
			choices: vec![ScatterChoice {
				kind: ShepherdsBuildingKind::House,
				weight: 1.0,
				min_footprint: 14.0,
				max_footprint: 23.0,
			}],
		};
		let homes = Self::scatter_shepherds(cell, confines.bounds.min.y, noise, &recipe);
		(!homes.is_empty()).then_some(SuburbanHomes { bounds: confines.bounds, homes })
	}

	pub(crate) fn build_old_city_market(
		cell: Aabb3d,
		confines: &Confines,
		noise: NoiseParams,
	) -> Option<OldCityMarket> {
		let recipe = ScatterRecipe {
			grid_side: 9,
			min_count: 40,
			max_count: 58,
			cell_inset: (cell.max.x - confines.bounds.max.x).abs() + 10.0,
			jitter: 8.0,
			clearance: 0.8,
			choices: vec![
				ScatterChoice {
					kind: ShepherdsBuildingKind::Hut,
					weight: 0.84,
					min_footprint: 5.0,
					max_footprint: 9.0,
				},
				ScatterChoice {
					kind: ShepherdsBuildingKind::House,
					weight: 0.16,
					min_footprint: 12.0,
					max_footprint: 19.0,
				},
			],
		};
		let buildings = Self::scatter_shepherds(cell, confines.bounds.min.y, noise, &recipe);
		(!buildings.is_empty()).then_some(OldCityMarket { bounds: confines.bounds, buildings })
	}

	pub(crate) fn build_temple_complex(
		cell: Aabb3d,
		confines: &Confines,
		noise: NoiseParams,
	) -> Option<TempleComplex> {
		let center = confines.center_xz();
		let y = confines.bounds.min.y;
		let extent = confines.footprint();
		let root = Self::root_hash(cell, noise);
		let mut halls = Vec::new();
		for (index, offset) in [
			Vec2::new(0.0, -extent.y * 0.31),
			Vec2::new(0.0, extent.y * 0.31),
			Vec2::new(-extent.x * 0.31, 0.0),
			Vec2::new(extent.x * 0.31, 0.0),
		]
		.into_iter()
		.enumerate()
		{
			let along_x = offset.x.abs() < offset.y.abs();
			let footprint = if along_x { Vec2::new(30.0, 18.0) } else { Vec2::new(18.0, 30.0) };
			let yaw = if along_x { 0.0 } else { std::f32::consts::FRAC_PI_2 };
			let hash = SeededHash::new(root.seed.wrapping_add(index as u32 * 97 + 1));
			let mut local_noise = noise;
			local_noise.seed = noise.seed.wrapping_add(index as i32 * 97);
			if let Some(hall) = fit_shepherds_building(
				ShepherdsBuildingKind::House,
				center + offset,
				yaw,
				footprint,
				y,
				hash,
				local_noise,
			) {
				halls.push(hall);
			}
		}

		let sanctum_footprint =
			Vec2::splat(extent.x.min(extent.y).mul_add(0.20, 0.0).clamp(28.0, 36.0));
		let sanctum_bounds = Aabb3d::from_min_max(
			Vec3::new(
				center.x - sanctum_footprint.x * 0.5,
				y,
				center.y - sanctum_footprint.y * 0.5,
			),
			Vec3::new(
				center.x + sanctum_footprint.x * 0.5,
				(y + 44.0).min(confines.bounds.max.y),
				center.y + sanctum_footprint.y * 0.5,
			),
		);
		let sanctum_confines = Confines::new(sanctum_bounds, 0.0, Openings::new());
		let (sanctum_building, _) =
			SingleHighrise::fit_to_confines(&sanctum_confines, noise).ok()?;
		let sanctum = PlacedBuilding {
			center_xz: center,
			yaw: 0.0,
			footprint: sanctum_footprint,
			ground_height: y,
			building: sanctum_building,
		};
		Some(TempleComplex { bounds: confines.bounds, halls, sanctum })
	}

	pub(crate) fn build_skybridge_bazaar(
		cell: Aabb3d,
		confines: &Confines,
		noise: NoiseParams,
	) -> Option<SkybridgeBazaar> {
		let center = confines.center_xz();
		let y = confines.bounds.min.y;
		let extent = confines.footprint();
		let spacing = (extent.x * 0.28).clamp(42.0, 60.0);
		let tower_foot = Vec2::splat((spacing * 0.48).clamp(24.0, 30.0));
		let mut towers = Vec::new();
		for (index, x) in [-spacing, 0.0, spacing].into_iter().enumerate() {
			let tower_center =
				center + Vec2::new(x, if index == 1 { extent.y * 0.12 } else { 0.0 });
			let bounds = Aabb3d::from_min_max(
				Vec3::new(
					tower_center.x - tower_foot.x * 0.5,
					y,
					tower_center.y - tower_foot.y * 0.5,
				),
				Vec3::new(
					tower_center.x + tower_foot.x * 0.5,
					confines.bounds.max.y,
					tower_center.y + tower_foot.y * 0.5,
				),
			);
			let (building, _) =
				SingleHighrise::fit_to_confines(&Confines::from_bounds(bounds), noise).ok()?;
			towers.push(PlacedBuilding {
				center_xz: tower_center,
				yaw: 0.0,
				footprint: tower_foot,
				ground_height: y,
				building,
			});
		}

		let bridge_y = y + 7.0 * richmond_developments::keep::TOWER_STOREY_HEIGHT;
		let mut bridges = Vec::new();
		for pair in towers.windows(2) {
			let a = pair[0].center_xz;
			let b = pair[1].center_xz;
			let bounds = Aabb3d::from_min_max(
				Vec3::new(a.x.min(b.x), bridge_y, a.y.min(b.y) - 3.0),
				Vec3::new(a.x.max(b.x), bridge_y + 4.2, a.y.max(b.y) + 3.0),
			);
			let bridge = Skybridge::new(bounds);
			bridges.push(PlacedBuilding {
				center_xz: (a + b) * 0.5,
				yaw: 0.0,
				footprint: Vec2::new(bounds.max.x - bounds.min.x, bounds.max.z - bounds.min.z),
				ground_height: bridge_y,
				building: bridge,
			});
		}

		let recipe = ScatterRecipe {
			grid_side: 5,
			min_count: 10,
			max_count: 16,
			cell_inset: (cell.max.x - confines.bounds.max.x).abs() + 24.0,
			jitter: 7.0,
			clearance: 1.5,
			choices: vec![ScatterChoice {
				kind: ShepherdsBuildingKind::Hut,
				weight: 1.0,
				min_footprint: 5.0,
				max_footprint: 8.0,
			}],
		};
		let market = Self::scatter_shepherds(cell, y, noise, &recipe)
			.into_iter()
			.filter(|building| {
				towers
					.iter()
					.all(|tower| building.center_xz.distance(tower.center_xz) > tower_foot.x)
			})
			.collect();

		Some(SkybridgeBazaar { bounds: confines.bounds, towers, bridges, market })
	}

	fn scatter_shepherds(
		cell: Aabb3d,
		height: f32,
		noise: NoiseParams,
		recipe: &ScatterRecipe<ShepherdsBuildingKind>,
	) -> Vec<richmond_developments::ShepherdsVillageBuilding> {
		let root = Self::root_hash(cell, noise);
		let plan = recipe.plan(cell, root);
		let mut occupied: Vec<Bounds2> = Vec::new();
		let mut buildings = Vec::new();
		for candidate in plan.candidates {
			if buildings.len() >= plan.target_count {
				break;
			}
			let collision = recipe.collision_bounds(&candidate);
			if occupied.iter().copied().any(|bounds| bounds_intersect(bounds, collision)) {
				continue;
			}
			let hash = SeededHash::new(
				root.seed.wrapping_add((candidate.slot as u32 + 1).wrapping_mul(0xA24B_AED5)),
			);
			let mut local_noise = noise;
			local_noise.seed = noise.seed.wrapping_add(candidate.slot as i32 * 97);
			if let Some(building) = fit_shepherds_building(
				candidate.kind,
				candidate.center,
				candidate.yaw,
				candidate.footprint,
				height,
				hash,
				local_noise,
			) {
				occupied.push(collision);
				buildings.push(building);
			}
		}
		buildings
	}

	fn root_hash(cell: Aabb3d, noise: NoiseParams) -> SeededHash {
		let salt = cell.min.x.to_bits().wrapping_mul(73856093)
			^ cell.min.z.to_bits().wrapping_mul(19349663);
		SeededHash::new((noise.seed as u32).wrapping_add(salt))
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use richmond_developments::ShepherdsBuilding;

	fn cell() -> Aabb3d {
		Aabb3d::from_min_max(Vec3::ZERO, Vec3::new(300.0, 1.0, 300.0))
	}

	fn confines(side: f32, height: f32) -> Confines {
		let center = Vec2::splat(150.0);
		Confines::from_bounds(Aabb3d::from_min_max(
			Vec3::new(center.x - side * 0.5, 10.0, center.y - side * 0.5),
			Vec3::new(center.x + side * 0.5, 10.0 + height, center.y + side * 0.5),
		))
	}

	#[test]
	fn campus_layouts_emit_landmarks_and_connections() -> anyhow::Result<()> {
		let cell = cell();
		let noise = NoiseParams::default();
		let temple = ArchetypeGenerator::build_temple_complex(cell, &confines(170.0, 60.0), noise)
			.ok_or_else(|| anyhow::anyhow!("temple did not fit"))?;
		assert_eq!(temple.halls.len(), 4);
		assert!(temple.sanctum.building.storey_count() >= 8);

		let bazaar =
			ArchetypeGenerator::build_skybridge_bazaar(cell, &confines(200.0, 80.0), noise)
				.ok_or_else(|| anyhow::anyhow!("bazaar did not fit"))?;
		assert_eq!(bazaar.towers.len(), 3);
		assert_eq!(bazaar.bridges.len(), 2);
		assert!(!bazaar.market.is_empty());
		Ok(())
	}

	#[test]
	fn old_city_market_is_denser_and_hut_led() -> anyhow::Result<()> {
		let cell = cell();
		let noise = NoiseParams::default();
		let suburban =
			ArchetypeGenerator::build_suburban_homes(cell, &confines(220.0, 14.0), noise)
				.ok_or_else(|| anyhow::anyhow!("suburb did not fit"))?;
		let market = ArchetypeGenerator::build_old_city_market(cell, &confines(220.0, 14.0), noise)
			.ok_or_else(|| anyhow::anyhow!("market did not fit"))?;
		let huts = market
			.buildings
			.iter()
			.filter(|building| matches!(building.building, ShepherdsBuilding::Hut(_)))
			.count();
		assert!(market.buildings.len() > suburban.homes.len() * 2);
		assert!(huts * 2 > market.buildings.len());
		Ok(())
	}
}
