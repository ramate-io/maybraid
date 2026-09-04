//! Builders for the shared solitary, campus, and neighborhood layout families.

use bevy::math::bounding::{Aabb2d, Aabb3d};
use bevy::math::{Vec2, Vec3};
use bevy::transform::components::Transform;
use procedural_common::{Bounds2, NoiseParams, SeededHash};
use richmond_buildings::{
	CardinalFace, Confines, ConnectingHall, Fit, MappedOpening, MappedOpeningQuad, Openings,
};
use richmond_developments::{
	PlacedBuilding, SingleHighrise, Skybridge, SkybridgeBazaar, SolitaryWizardsTower,
	SuburbanHomes, TempleComplex, TempleSanctum,
};

use crate::cell::yaw_about_xz;
use crate::finish::{DevelopmentFinish, DevelopmentFinishRole, SuburbanPaletteBias};
use crate::scatter::{bounds_intersect, ScatterChoice, ScatterRecipe};
use crate::shepherds_fit::{
	fit_shepherds_building, fit_shepherds_building_for_role, fit_suburban_building,
	ShepherdsBuildingKind,
};

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
		let bounds = Aabb2d {
			min: Vec2::new(confines.bounds.min.x, confines.bounds.min.z),
			max: Vec2::new(confines.bounds.max.x, confines.bounds.max.z),
		};
		let root = Self::root_hash(cell, noise);
		let bias = SuburbanPaletteBias::select(root);
		let homes_recipe = ScatterRecipe {
			grid_side: 4,
			min_count: 8,
			max_count: 10,
			cell_inset: 27.0,
			jitter: 7.0,
			clearance: 8.0,
			choices: vec![ScatterChoice {
				kind: ShepherdsBuildingKind::House,
				weight: 1.0,
				min_footprint: 14.0,
				max_footprint: 23.0,
			}],
		};
		let (homes, occupied) = Self::scatter_suburban(
			bounds,
			confines.bounds.min.y,
			noise,
			&homes_recipe,
			root,
			bias,
			Vec::new(),
		);
		if homes.is_empty() {
			return None;
		}

		let secondary_recipe = ScatterRecipe {
			grid_side: 5,
			min_count: 2,
			max_count: 3,
			cell_inset: 15.0,
			jitter: 9.0,
			clearance: 5.0,
			choices: vec![ScatterChoice {
				kind: ShepherdsBuildingKind::Hut,
				weight: 1.0,
				min_footprint: 5.0,
				max_footprint: 8.0,
			}],
		};
		let secondary_root = SeededHash::new(root.seed.wrapping_add(0x6A09_E667));
		let (secondary_buildings, _) = Self::scatter_suburban(
			bounds,
			confines.bounds.min.y,
			noise,
			&secondary_recipe,
			secondary_root,
			bias,
			occupied,
		);
		Some(SuburbanHomes { bounds: confines.bounds, homes, secondary_buildings })
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
			if let Some(hall) = fit_shepherds_building_for_role(
				ShepherdsBuildingKind::House,
				center + offset,
				yaw,
				footprint,
				y,
				hash,
				local_noise,
				DevelopmentFinishRole::Temple,
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
			TempleSanctum::fit_to_confines(&sanctum_confines, noise).ok()?;
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
		let root = Self::root_hash(cell, noise);
		let tower_material =
			DevelopmentFinish::pick_for_role(root, DevelopmentFinishRole::Highrise, false).wall;
		let bridge_material =
			DevelopmentFinish::pick_for_role(root, DevelopmentFinishRole::Connector, false).wall;
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
				building: building.with_wall_material(tower_material.clone()),
			});
		}

		let mut bridges = Vec::new();
		for (pair_index, pair) in towers.windows(2).enumerate() {
			let a = &pair[0];
			let b = &pair[1];
			let common_storeys = a.building.storey_count().min(b.building.storey_count());
			let bridge_storey = Self::bridge_storey(root, pair_index, common_storeys)?;
			let direction_a = Self::facing_cardinal(a, b.center_xz - a.center_xz);
			let direction_b = Self::facing_cardinal(b, a.center_xz - b.center_xz);
			let end_a = Self::mapped_bridge_endpoint(a, bridge_storey, direction_a)?
				.widened(0.6)
				.raised(0.75);
			let end_b = Self::mapped_bridge_endpoint(b, bridge_storey, direction_b)?
				.widened(0.6)
				.raised(0.75);
			let hall = ConnectingHall::rough_stone(end_a, end_b);
			let bridge = Skybridge::new(hall, bridge_storey, [direction_a, direction_b])
				.with_material(bridge_material.clone());
			let bounds = bridge.bounds;
			let bridge_center =
				Vec2::new((bounds.min.x + bounds.max.x) * 0.5, (bounds.min.z + bounds.max.z) * 0.5);
			bridges.push(PlacedBuilding {
				center_xz: bridge_center,
				yaw: 0.0,
				footprint: Vec2::new(bounds.max.x - bounds.min.x, bounds.max.z - bounds.min.z),
				ground_height: bounds.min.y,
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

	fn bridge_storey(root: SeededHash, pair_index: usize, common_storeys: usize) -> Option<usize> {
		let first = 2;
		let count = common_storeys.checked_sub(4)?;
		let offset = (root.unit(701 + pair_index as u32) * count as f32).floor() as usize;
		Some(first + offset.min(count.saturating_sub(1)))
	}

	fn facing_cardinal(
		tower: &PlacedBuilding<SingleHighrise>,
		world_direction: Vec2,
	) -> CardinalFace {
		let local = Self::rotate_xz(world_direction, -tower.yaw);
		if local.x.abs() >= local.y.abs() {
			if local.x >= 0.0 {
				CardinalFace::East
			} else {
				CardinalFace::West
			}
		} else if local.y >= 0.0 {
			CardinalFace::North
		} else {
			CardinalFace::South
		}
	}

	fn mapped_bridge_endpoint(
		tower: &PlacedBuilding<SingleHighrise>,
		storey: usize,
		direction: CardinalFace,
	) -> Option<MappedOpening> {
		let mapped = *tower.building.mapped_bridge_passage(storey, direction)?;
		let transform = yaw_about_xz(tower.center_xz, tower.yaw);
		let (bl, br, tl, tr) = mapped.endpoint_corners();
		Some(MappedOpening::new(
			MappedOpeningQuad::new(
				transform.transform_point(bl),
				transform.transform_point(br),
				transform.transform_point(tl),
				transform.transform_point(tr),
			),
			Self::rotate_xz(mapped.orientation, tower.yaw),
		))
	}

	fn rotate_xz(vector: Vec2, yaw: f32) -> Vec2 {
		let (sin, cos) = yaw.sin_cos();
		Vec2::new(cos * vector.x + sin * vector.y, -sin * vector.x + cos * vector.y)
	}

	fn scatter_shepherds(
		cell: Aabb3d,
		height: f32,
		noise: NoiseParams,
		recipe: &ScatterRecipe<ShepherdsBuildingKind>,
	) -> Vec<richmond_developments::ShepherdsVillageBuilding> {
		Self::scatter_shepherds_with_role(cell, height, noise, recipe, None)
	}

	fn scatter_shepherds_with_role(
		cell: Aabb3d,
		height: f32,
		noise: NoiseParams,
		recipe: &ScatterRecipe<ShepherdsBuildingKind>,
		role: Option<DevelopmentFinishRole>,
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
			let building = if let Some(role) = role {
				fit_shepherds_building_for_role(
					candidate.kind,
					candidate.center,
					candidate.yaw,
					candidate.footprint,
					height,
					hash,
					local_noise,
					role,
				)
			} else {
				fit_shepherds_building(
					candidate.kind,
					candidate.center,
					candidate.yaw,
					candidate.footprint,
					height,
					hash,
					local_noise,
				)
			};
			if let Some(building) = building {
				occupied.push(collision);
				buildings.push(building);
			}
		}
		buildings
	}

	fn scatter_suburban(
		bounds: Aabb2d,
		height: f32,
		noise: NoiseParams,
		recipe: &ScatterRecipe<ShepherdsBuildingKind>,
		root: SeededHash,
		bias: SuburbanPaletteBias,
		mut occupied: Vec<Bounds2>,
	) -> (Vec<richmond_developments::ShepherdsVillageBuilding>, Vec<Bounds2>) {
		let initial_occupied = occupied.len();
		let plan = recipe.plan_in_bounds(bounds, root);
		let mut buildings = Vec::new();
		for candidate in plan.candidates {
			if buildings.len() >= plan.target_count {
				break;
			}
			let collision = recipe.collision_bounds(&candidate);
			if !Self::bounds_contains(bounds, collision)
				|| occupied.iter().copied().any(|other| bounds_intersect(other, collision))
			{
				continue;
			}
			let hash = SeededHash::new(
				root.seed.wrapping_add((candidate.slot as u32 + 1).wrapping_mul(0xA24B_AED5)),
			);
			let mut local_noise = noise;
			local_noise.seed = noise.seed.wrapping_add(candidate.slot as i32 * 97);
			if let Some(building) = fit_suburban_building(
				candidate.kind,
				candidate.center,
				candidate.yaw,
				candidate.footprint,
				height,
				hash,
				local_noise,
				bias,
			) {
				occupied.push(collision);
				buildings.push(building);
			}
		}
		debug_assert_eq!(occupied.len(), initial_occupied + buildings.len());
		(buildings, occupied)
	}

	fn bounds_contains(bounds: Aabb2d, inner: Bounds2) -> bool {
		inner.min.x >= bounds.min.x
			&& inner.max.x <= bounds.max.x
			&& inner.min.y >= bounds.min.y
			&& inner.max.y <= bounds.max.y
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
	use lod::gen::LodSceneLevel;
	use richmond_building_components::BuildingComponents;
	use richmond_developments::{ShepherdsBuilding, ShepherdsFinish};

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
		assert!(temple.sanctum.building.recipe_index() < TempleSanctum::RECIPE_COUNT);

		let bazaar =
			ArchetypeGenerator::build_skybridge_bazaar(cell, &confines(200.0, 80.0), noise)
				.ok_or_else(|| anyhow::anyhow!("bazaar did not fit"))?;
		assert_eq!(bazaar.towers.len(), 3);
		assert_eq!(bazaar.bridges.len(), 2);
		assert!(!bazaar.market.is_empty());
		for (index, bridge) in bazaar.bridges.iter().enumerate() {
			let pair = &bazaar.towers[index..=index + 1];
			assert!(bridge.building.storey < pair[0].building.storey_count());
			assert!(bridge.building.storey < pair[1].building.storey_count());
			assert_eq!(bridge.building.directions, [CardinalFace::East, CardinalFace::West]);
			let expected_a = ArchetypeGenerator::mapped_bridge_endpoint(
				&pair[0],
				bridge.building.storey,
				CardinalFace::East,
			)
			.ok_or_else(|| anyhow::anyhow!("left tower endpoint was not mapped"))?
			.widened(0.6)
			.raised(0.75);
			let expected_b = ArchetypeGenerator::mapped_bridge_endpoint(
				&pair[1],
				bridge.building.storey,
				CardinalFace::West,
			)
			.ok_or_else(|| anyhow::anyhow!("right tower endpoint was not mapped"))?
			.widened(0.6)
			.raised(0.75);
			assert_eq!(bridge.building.endpoints(), (expected_a, expected_b));
			assert!(!bridge.building.panel_nodes_for_level(LodSceneLevel::High).is_empty());
			assert!(bridge.building.floor_nodes_for_level(LodSceneLevel::High).is_empty());
		}
		Ok(())
	}

	#[test]
	fn skybridge_storeys_endpoints_and_materials_are_deterministic() -> anyhow::Result<()> {
		let cell = cell();
		let confines = confines(200.0, 80.0);
		let noise = NoiseParams { seed: 83, ..NoiseParams::default() };
		let first = ArchetypeGenerator::build_skybridge_bazaar(cell, &confines, noise)
			.ok_or_else(|| anyhow::anyhow!("first bazaar did not fit"))?;
		let second = ArchetypeGenerator::build_skybridge_bazaar(cell, &confines, noise)
			.ok_or_else(|| anyhow::anyhow!("second bazaar did not fit"))?;
		assert_eq!(first.bridges, second.bridges);
		for bridge in &first.bridges {
			let material = bridge
				.building
				.material()
				.ok_or_else(|| anyhow::anyhow!("bridge material was not stamped"))?;
			let panels = bridge.building.panel_nodes_for_level(LodSceneLevel::High).flatten();
			assert!(!panels.is_empty());
			assert!(panels.iter().all(|panel| panel.material.as_ref() == Some(material)));
		}
		Ok(())
	}

	fn shepherds_finish(
		building: &richmond_developments::ShepherdsVillageBuilding,
	) -> anyhow::Result<&ShepherdsFinish> {
		match &building.building {
			ShepherdsBuilding::House(house) => house.finish.as_ref(),
			ShepherdsBuilding::Hut(hut) => hut.finish.as_ref(),
		}
		.ok_or_else(|| anyhow::anyhow!("suburban building had no finish"))
	}

	#[test]
	fn suburban_homes_are_coherent_diverse_and_include_outbuildings() -> anyhow::Result<()> {
		let cell = cell();
		let confines = confines(210.0, 16.0);
		let noise = NoiseParams { seed: 29, ..NoiseParams::default() };
		let first = ArchetypeGenerator::build_suburban_homes(cell, &confines, noise)
			.ok_or_else(|| anyhow::anyhow!("first neighborhood did not fit"))?;
		let second = ArchetypeGenerator::build_suburban_homes(cell, &confines, noise)
			.ok_or_else(|| anyhow::anyhow!("second neighborhood did not fit"))?;
		let first_buildings: Vec<_> = first.buildings().collect();
		let second_buildings: Vec<_> = second.buildings().collect();
		assert_eq!(first_buildings.len(), second_buildings.len());
		for (a, b) in first_buildings.iter().zip(&second_buildings) {
			assert_eq!(a.center_xz, b.center_xz);
			assert_eq!(a.yaw, b.yaw);
			assert_eq!(a.footprint, b.footprint);
			assert_eq!(shepherds_finish(a)?, shepherds_finish(b)?);
		}
		assert!(first.homes.len() >= 7);
		assert!(!first.secondary_buildings.is_empty());
		assert!(first
			.secondary_buildings
			.iter()
			.all(|building| matches!(&building.building, ShepherdsBuilding::Hut(_))));

		let finishes: Vec<_> =
			first.homes.iter().map(shepherds_finish).collect::<anyhow::Result<_>>()?;
		let distinct = finishes
			.iter()
			.enumerate()
			.filter(|(index, finish)| finishes[..*index].iter().all(|earlier| *earlier != **finish))
			.count();
		assert!(distinct > 1, "representative neighborhood should vary house finishes");

		let bounds = Aabb2d {
			min: Vec2::new(first.bounds.min.x, first.bounds.min.z),
			max: Vec2::new(first.bounds.max.x, first.bounds.max.z),
		};
		for (index, building) in first_buildings.iter().enumerate() {
			let half = crate::cell::yawed_plan_aabb_extent(
				building.footprint.x,
				building.footprint.y,
				building.yaw,
			) * 0.5;
			let footprint =
				Bounds2 { min: building.center_xz - half, max: building.center_xz + half };
			assert!(ArchetypeGenerator::bounds_contains(bounds, footprint));
			for other in &first_buildings[..index] {
				let other_half = crate::cell::yawed_plan_aabb_extent(
					other.footprint.x,
					other.footprint.y,
					other.yaw,
				) * 0.5;
				let other_footprint = Bounds2 {
					min: other.center_xz - other_half,
					max: other.center_xz + other_half,
				};
				assert!(!bounds_intersect(footprint, other_footprint));
			}
		}
		Ok(())
	}
}
