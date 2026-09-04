//! Uniform presentation descriptors for every development building host.

use std::sync::Arc;

use bevy::prelude::{
	bsn, template_value, Commands, CommandsSceneExt, Entity, Transform, Visibility,
};
use lod::gen::LodScene;
use lod::lod_host_scene_pending;
use lod::lod_ref::LodRef;
use richmond_building_components::{
	building_bounds, spawn_building_components, BuildingComponents,
};
use richmond_buildings::wizards_tower::WizardsTower;
use richmond_buildings::{
	ConnectingStairwell, MixedUseLesHallesStorey, PitchedRoof, RectangularPitchedRoofComplex,
};
use richmond_developments::{
	CircularTower, GalleryColonnade, GalleryTerrace, MixedUseLesHallesHost, OldCityMarketTerrace,
	RingFortHost, ShepherdsBuilding, ShepherdsHouse, ShepherdsHut, SingleHighrise, Skybridge,
	TempleSanctum, TrazaloidTower,
};

use crate::cell::yaw_about_xz;
use crate::{
	BuiltDevelopment, LesHallesDevelopment, RingFortDevelopment, ShepherdsCommuneDevelopment,
	ShepherdsVillageDevelopment,
};

#[derive(Debug, Clone)]
pub enum DevelopmentHost {
	LesHallesStorey(Arc<MixedUseLesHallesStorey>, Transform),
	LesHallesStairwell(Box<ConnectingStairwell>, Transform),
	LesHallesRoof(Box<PitchedRoof>, Transform),
	ShepherdsHouse(Arc<ShepherdsHouse>, Transform),
	ShepherdsHut(Arc<ShepherdsHut>, Transform),
	OldCityMarketTerrace(Arc<OldCityMarketTerrace>, Transform),
	RingFortCircularTower(Arc<CircularTower>, Transform),
	RingFortTrazaloidTower(Arc<TrazaloidTower>, Transform),
	RingFortGalleryTerrace(Box<GalleryTerrace>, Transform),
	RingFortGalleryColonnade(Box<GalleryColonnade>, Transform),
	RingFortGalleryRoof(Box<RectangularPitchedRoofComplex>, Transform),
	SingleHighrise(Arc<SingleHighrise>, Transform),
	TempleSanctum(Arc<TempleSanctum>, Transform),
	WizardsTower(Arc<WizardsTower>, Transform),
	SkybridgeHall(Arc<Skybridge>, Transform),
}

impl DevelopmentHost {
	pub fn spawn(&self, commands: &mut Commands) -> Vec<Entity> {
		match self {
			Self::LesHallesStorey(building, transform) => spawn(commands, building, *transform),
			Self::LesHallesStairwell(building, transform) => {
				spawn(commands, building.as_ref(), *transform)
			}
			Self::LesHallesRoof(building, transform) => {
				spawn(commands, building.as_ref(), *transform)
			}
			Self::ShepherdsHouse(building, transform) => spawn(commands, building, *transform),
			Self::ShepherdsHut(building, transform) => spawn(commands, building, *transform),
			Self::OldCityMarketTerrace(building, transform) => {
				spawn(commands, building, *transform)
			}
			Self::RingFortCircularTower(building, transform) => {
				spawn(commands, building, *transform)
			}
			Self::RingFortTrazaloidTower(building, transform) => {
				spawn(commands, building, *transform)
			}
			Self::RingFortGalleryTerrace(building, transform) => {
				spawn(commands, building.as_ref(), *transform)
			}
			Self::RingFortGalleryColonnade(building, transform) => {
				spawn(commands, building.as_ref(), *transform)
			}
			Self::RingFortGalleryRoof(building, transform) => {
				spawn(commands, building.as_ref(), *transform)
			}
			Self::SingleHighrise(building, transform) => spawn(commands, building, *transform),
			Self::TempleSanctum(building, transform) => spawn(commands, building, *transform),
			Self::WizardsTower(building, transform) => {
				spawn_wizards_tower(commands, building, *transform)
			}
			Self::SkybridgeHall(building, transform) => spawn(commands, building, *transform),
		}
	}
}

pub trait DevelopmentHosts {
	fn hosts(&self) -> Vec<DevelopmentHost>;
}

impl DevelopmentHosts for BuiltDevelopment {
	fn hosts(&self) -> Vec<DevelopmentHost> {
		match self {
			Self::LesHalles(development) => development.hosts(),
			Self::ShepherdsVillage(development) => development.hosts(),
			Self::ShepherdsCommune(development) => development.hosts(),
			Self::RingFort(development) => development.hosts(),
			Self::TempleComplex(development) => {
				let mut hosts = shepherd_building_hosts(&development.halls);
				hosts.push(DevelopmentHost::TempleSanctum(
					Arc::new(development.sanctum.building.clone()),
					yaw_about_xz(development.sanctum.center_xz, development.sanctum.yaw),
				));
				hosts
			}
			Self::SingleHighrise(development) => {
				vec![single_highrise_host(&development.building)]
			}
			Self::SuburbanHomes(development) => shepherd_building_hosts(development.buildings()),
			Self::WizardsTower(development) => vec![DevelopmentHost::WizardsTower(
				Arc::new(development.building.building.tower.clone()),
				development.host_transform(),
			)],
			Self::SkybridgeBazaar(development) => {
				let mut hosts = shepherd_building_hosts(&development.market);
				hosts.extend(development.towers.iter().map(single_highrise_host));
				hosts.extend(development.bridges.iter().map(|placed| {
					DevelopmentHost::SkybridgeHall(
						Arc::new(placed.building.clone()),
						yaw_about_xz(placed.center_xz, placed.yaw),
					)
				}));
				hosts
			}
			Self::OldCityMarket(development) => {
				let mut hosts = shepherd_building_hosts(development.buildings());
				hosts.extend(development.terraces().map(|terrace| {
					DevelopmentHost::OldCityMarketTerrace(
						Arc::new(terrace.building.clone()),
						yaw_about_xz(terrace.center_xz, terrace.yaw),
					)
				}));
				hosts
			}
		}
	}
}

impl DevelopmentHosts for LesHallesDevelopment {
	fn hosts(&self) -> Vec<DevelopmentHost> {
		let transform = self.host_transform();
		self.building
			.building
			.hosts()
			.into_iter()
			.map(|host| match host {
				MixedUseLesHallesHost::Storey(storey) => {
					DevelopmentHost::LesHallesStorey(Arc::new(storey), transform)
				}
				MixedUseLesHallesHost::Stairwell(stairwell) => {
					DevelopmentHost::LesHallesStairwell(Box::new(stairwell), transform)
				}
				MixedUseLesHallesHost::Roof(roof) => {
					DevelopmentHost::LesHallesRoof(Box::new(roof), transform)
				}
			})
			.collect()
	}
}

impl DevelopmentHosts for ShepherdsVillageDevelopment {
	fn hosts(&self) -> Vec<DevelopmentHost> {
		shepherd_building_hosts(&self.village.buildings)
	}
}

impl DevelopmentHosts for ShepherdsCommuneDevelopment {
	fn hosts(&self) -> Vec<DevelopmentHost> {
		shepherd_building_hosts(self.commune.buildings())
	}
}

impl DevelopmentHosts for RingFortDevelopment {
	fn hosts(&self) -> Vec<DevelopmentHost> {
		let transform = self.host_transform();
		self.building
			.building
			.hosts()
			.into_iter()
			.filter_map(|host| match host {
				RingFortHost::Ring(host) => match *host {
					MixedUseLesHallesHost::Storey(storey) => {
						Some(DevelopmentHost::LesHallesStorey(Arc::new(storey), transform))
					}
					MixedUseLesHallesHost::Stairwell(stairwell) => {
						Some(DevelopmentHost::LesHallesStairwell(Box::new(stairwell), transform))
					}
					MixedUseLesHallesHost::Roof(_) => None,
				},
				RingFortHost::Circular(tower) => {
					Some(DevelopmentHost::RingFortCircularTower(tower, transform))
				}
				RingFortHost::Trazaloid(tower) => {
					Some(DevelopmentHost::RingFortTrazaloidTower(tower, transform))
				}
				RingFortHost::Terrace(terrace) => {
					Some(DevelopmentHost::RingFortGalleryTerrace(Box::new(terrace), transform))
				}
				RingFortHost::TerraceStairwell(stairwell)
				| RingFortHost::KeepStairwell(stairwell) => {
					Some(DevelopmentHost::LesHallesStairwell(Box::new(stairwell), transform))
				}
				RingFortHost::GalleryColonnade(colonnade) => {
					Some(DevelopmentHost::RingFortGalleryColonnade(Box::new(colonnade), transform))
				}
				RingFortHost::GalleryRoof(roof) => {
					Some(DevelopmentHost::RingFortGalleryRoof(Box::new(roof), transform))
				}
			})
			.collect()
	}
}

fn shepherd_building_hosts<'a>(
	buildings: impl IntoIterator<Item = &'a richmond_developments::ShepherdsVillageBuilding>,
) -> Vec<DevelopmentHost> {
	buildings
		.into_iter()
		.map(|placed| {
			let transform = yaw_about_xz(placed.center_xz, placed.yaw);
			match &placed.building {
				ShepherdsBuilding::House(house) => {
					DevelopmentHost::ShepherdsHouse(house.clone(), transform)
				}
				ShepherdsBuilding::Hut(hut) => {
					DevelopmentHost::ShepherdsHut(hut.clone(), transform)
				}
			}
		})
		.collect()
}

fn single_highrise_host(
	placed: &richmond_developments::PlacedBuilding<SingleHighrise>,
) -> DevelopmentHost {
	DevelopmentHost::SingleHighrise(
		Arc::new(placed.building.clone()),
		yaw_about_xz(placed.center_xz, placed.yaw),
	)
}

fn spawn_wizards_tower(
	commands: &mut Commands,
	building: &Arc<WizardsTower>,
	transform: Transform,
) -> Vec<Entity> {
	let bounds = building.scene_bounds();
	let identity = Transform::IDENTITY;
	let lod_ref = LodRef {
		entity: Entity::PLACEHOLDER,
		previous_transform: &identity,
		current_transform: &identity,
		bounds: &bounds,
	};
	let level = building.scene_lod_level(&lod_ref);
	let entity = commands
		.spawn_scene((
			lod_host_scene_pending(level, bounds),
			bsn! {
				template_value(transform)
				Visibility::default()
			},
		))
		.id();
	commands.entity(entity).insert(building.as_ref().clone());
	vec![entity]
}

fn spawn<T>(commands: &mut Commands, building: &T, transform: Transform) -> Vec<Entity>
where
	T: BuildingComponents + Clone + Send + Sync + 'static,
{
	let bounds = building_bounds(building);
	spawn_building_components(commands, building, transform, bounds)
}

#[cfg(test)]
mod tests {
	use bevy::math::bounding::Aabb3d;
	use bevy::math::{Vec2, Vec3};
	use procedural_common::NoiseParams;
	use richmond_buildings::{Confines, Fit};
	use richmond_developments::PlacedBuilding;

	use super::{DevelopmentHost, DevelopmentHosts};
	use crate::archetype_generation::{ArchetypeGenerator, PlacedDevelopment};
	use crate::BuiltDevelopment;

	#[test]
	fn placed_single_highrise_emits_exactly_one_host() -> anyhow::Result<()> {
		let cell = Aabb3d::from_min_max(Vec3::new(-30.0, 0.0, -30.0), Vec3::new(30.0, 64.0, 30.0));
		let confines = Confines::from_bounds(cell);
		let (building, _) = richmond_developments::SingleHighrise::fit_to_confines(
			&confines,
			NoiseParams::default(),
		)?;
		let development = BuiltDevelopment::SingleHighrise(Box::new(PlacedDevelopment {
			cell,
			building: PlacedBuilding {
				center_xz: Vec2::ZERO,
				yaw: 0.0,
				footprint: Vec2::splat(60.0),
				ground_height: 0.0,
				building,
			},
		}));
		let hosts = development.hosts();
		assert_eq!(hosts.len(), 1);
		assert!(matches!(hosts[0], DevelopmentHost::SingleHighrise(..)));
		Ok(())
	}

	#[test]
	fn suburban_and_skybridge_hosts_stay_bounded_and_include_new_buildings() -> anyhow::Result<()> {
		let cell = Aabb3d::from_min_max(Vec3::ZERO, Vec3::new(300.0, 1.0, 300.0));
		let suburban_confines = Confines::from_bounds(Aabb3d::from_min_max(
			Vec3::new(45.0, 10.0, 45.0),
			Vec3::new(255.0, 26.0, 255.0),
		));
		let suburban = ArchetypeGenerator::build_suburban_homes(
			cell,
			&suburban_confines,
			NoiseParams { seed: 29, ..NoiseParams::default() },
		)
		.ok_or_else(|| anyhow::anyhow!("suburban neighborhood did not fit"))?;
		let expected = suburban.buildings().count();
		let hosts = BuiltDevelopment::SuburbanHomes(Box::new(suburban)).hosts();
		assert_eq!(hosts.len(), expected);
		assert!(hosts.len() <= 13);
		assert!(hosts.iter().all(|host| matches!(
			host,
			DevelopmentHost::ShepherdsHouse(..) | DevelopmentHost::ShepherdsHut(..)
		)));

		let bazaar_confines = Confines::from_bounds(Aabb3d::from_min_max(
			Vec3::new(50.0, 10.0, 50.0),
			Vec3::new(250.0, 90.0, 250.0),
		));
		let bazaar = ArchetypeGenerator::build_skybridge_bazaar(
			cell,
			&bazaar_confines,
			NoiseParams::default(),
		)
		.ok_or_else(|| anyhow::anyhow!("skybridge bazaar did not fit"))?;
		let expected = bazaar.market.len() + bazaar.towers.len() + bazaar.bridges.len();
		let hosts = BuiltDevelopment::SkybridgeBazaar(Box::new(bazaar)).hosts();
		assert_eq!(hosts.len(), expected);
		assert!(hosts.len() <= 21);
		assert_eq!(
			hosts
				.iter()
				.filter(|host| matches!(host, DevelopmentHost::SkybridgeHall(..)))
				.count(),
			2
		);
		Ok(())
	}
}
