//! Uniform presentation descriptors for every development building host.

use std::sync::Arc;

use bevy::prelude::{Commands, Entity, Transform};
use richmond_building_components::{
	building_bounds, spawn_building_components, BuildingComponents,
};
use richmond_buildings::{ArcTower, ConnectingStairwell, MixedUseLesHallesStorey, PitchedRoof};
use richmond_developments::{
	MixedUseLesHallesHost, RingFortHost, ShepherdsBuilding, ShepherdsHouse, ShepherdsHut,
	TrazaloidTower,
};

use crate::cell::yaw_about_xz;
use crate::{
	LesHallesDevelopment, RingFortDevelopment, ShepherdsCommuneDevelopment,
	ShepherdsVillageDevelopment,
};

#[derive(Debug, Clone)]
pub enum DevelopmentHost {
	LesHallesStorey(Arc<MixedUseLesHallesStorey>, Transform),
	LesHallesStairwell(ConnectingStairwell, Transform),
	LesHallesRoof(PitchedRoof, Transform),
	ShepherdsHouse(Arc<ShepherdsHouse>, Transform),
	ShepherdsHut(Arc<ShepherdsHut>, Transform),
	RingFortCircularTower(Arc<ArcTower>, Transform),
	RingFortTrazaloidTower(Arc<TrazaloidTower>, Transform),
}

impl DevelopmentHost {
	pub fn spawn(&self, commands: &mut Commands) -> Vec<Entity> {
		match self {
			Self::LesHallesStorey(building, transform) => spawn(commands, building, *transform),
			Self::LesHallesStairwell(building, transform) => spawn(commands, building, *transform),
			Self::LesHallesRoof(building, transform) => spawn(commands, building, *transform),
			Self::ShepherdsHouse(building, transform) => spawn(commands, building, *transform),
			Self::ShepherdsHut(building, transform) => spawn(commands, building, *transform),
			Self::RingFortCircularTower(building, transform) => {
				spawn(commands, building, *transform)
			}
			Self::RingFortTrazaloidTower(building, transform) => {
				spawn(commands, building, *transform)
			}
		}
	}
}

pub trait DevelopmentHosts {
	fn hosts(&self) -> Vec<DevelopmentHost>;
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
					DevelopmentHost::LesHallesStairwell(stairwell, transform)
				}
				MixedUseLesHallesHost::Roof(roof) => {
					DevelopmentHost::LesHallesRoof(roof, transform)
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
			.map(|host| match host {
				RingFortHost::Ring(host) => match *host {
					MixedUseLesHallesHost::Storey(storey) => {
						DevelopmentHost::LesHallesStorey(Arc::new(storey), transform)
					}
					MixedUseLesHallesHost::Stairwell(stairwell) => {
						DevelopmentHost::LesHallesStairwell(stairwell, transform)
					}
					MixedUseLesHallesHost::Roof(roof) => {
						DevelopmentHost::LesHallesRoof(roof, transform)
					}
				},
				RingFortHost::Circular(tower) => {
					DevelopmentHost::RingFortCircularTower(tower, transform)
				}
				RingFortHost::Trazaloid(tower) => {
					DevelopmentHost::RingFortTrazaloidTower(tower, transform)
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

fn spawn<T>(commands: &mut Commands, building: &T, transform: Transform) -> Vec<Entity>
where
	T: BuildingComponents + Clone + Send + Sync + 'static,
{
	let bounds = building_bounds(building);
	spawn_building_components(commands, building, transform, bounds)
}
