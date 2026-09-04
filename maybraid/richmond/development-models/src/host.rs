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
	CircularTower, GalleryColonnade, GalleryTerrace, MixedUseLesHallesHost, RingFortHost,
	ShepherdsBuilding, ShepherdsHouse, ShepherdsHut, SingleHighrise, Skybridge, TrazaloidTower,
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
	RingFortCircularTower(Arc<CircularTower>, Transform),
	RingFortTrazaloidTower(Arc<TrazaloidTower>, Transform),
	RingFortGalleryTerrace(Box<GalleryTerrace>, Transform),
	RingFortGalleryColonnade(Box<GalleryColonnade>, Transform),
	RingFortGalleryRoof(Box<RectangularPitchedRoofComplex>, Transform),
	SingleHighrise(Arc<SingleHighrise>, Transform),
	WizardsTower(Arc<WizardsTower>, Transform),
	Skybridge(Arc<Skybridge>, Transform),
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
			Self::WizardsTower(building, transform) => {
				spawn_wizards_tower(commands, building, *transform)
			}
			Self::Skybridge(building, transform) => spawn(commands, building, *transform),
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
				hosts.push(single_highrise_host(&development.sanctum));
				hosts
			}
			Self::SingleHighrise(development) => {
				vec![single_highrise_host(&development.building)]
			}
			Self::SuburbanHomes(development) => shepherd_building_hosts(&development.homes),
			Self::WizardsTower(development) => vec![DevelopmentHost::WizardsTower(
				Arc::new(development.building.building.tower.clone()),
				development.host_transform(),
			)],
			Self::SkybridgeBazaar(development) => {
				let mut hosts = shepherd_building_hosts(&development.market);
				hosts.extend(development.towers.iter().map(single_highrise_host));
				hosts.extend(development.bridges.iter().map(|placed| {
					DevelopmentHost::Skybridge(
						Arc::new(placed.building.clone()),
						yaw_about_xz(placed.center_xz, placed.yaw),
					)
				}));
				hosts
			}
			Self::OldCityMarket(development) => shepherd_building_hosts(&development.buildings),
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
