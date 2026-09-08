//! Reusable individual intelligence profiles assembled by `npc-intelligence`.

use bevy::prelude::Component;
use npc_intelligence::Personality;
use poi_intelligence::{PoiInterest, PoiInterests, PoiKind};

use crate::number::{index, FromMobNumber};

pub const VEGETATION_POI: PoiKind = PoiKind::new("mobs/vegetation");
pub const CHARACTER_POI: PoiKind = PoiKind::new("mobs/character");
pub const URBAN_POI: PoiKind = PoiKind::new("mobs/urban");
pub const LOCAL_POI: PoiKind = PoiKind::new("mobs/local");
pub const SALOON_POI: PoiKind = PoiKind::new("mobs/saloon");

#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CharacterBrains {
	#[default]
	Grazinger,
	PackHunter,
	Raider,
	Guard,
	Civilian,
	Roamer,
	Brawler,
}

impl CharacterBrains {
	pub const VALUES: [Self; 7] = [
		Self::Grazinger,
		Self::PackHunter,
		Self::Raider,
		Self::Guard,
		Self::Civilian,
		Self::Roamer,
		Self::Brawler,
	];

	pub const fn personality(self, armed: bool) -> Personality {
		match self {
			Self::PackHunter => Personality::Predator,
			Self::Raider => Personality::Assassin,
			Self::Guard | Self::Brawler => Personality::Brawler,
			Self::Civilian => Personality::Civilian,
			Self::Roamer if armed => Personality::Brawler,
			Self::Grazinger | Self::Roamer => Personality::Grazer,
		}
	}

	pub fn interests(self) -> PoiInterests {
		self.interests_for_slot(0)
	}

	/// Role-split table so even/odd guards and brawlers do not share one URBAN max.
	pub fn interests_for_slot(self, slot: usize) -> PoiInterests {
		let interests = match self {
			Self::Grazinger => PoiInterests::one(VEGETATION_POI),
			Self::PackHunter => PoiInterests::new([
				PoiInterest::new(CHARACTER_POI, 1.5),
				PoiInterest::new(VEGETATION_POI, 0.35),
			]),
			Self::Raider => PoiInterests::new([
				PoiInterest::new(CHARACTER_POI, 1.2),
				PoiInterest::new(URBAN_POI, 1.0),
			]),
			Self::Guard => PoiInterests::new([
				PoiInterest::new(URBAN_POI, 1.25),
				PoiInterest::new(LOCAL_POI, 0.9),
				PoiInterest::new(CHARACTER_POI, 0.7),
			]),
			Self::Civilian => PoiInterests::one(LOCAL_POI),
			Self::Roamer => PoiInterests::new([
				PoiInterest::new(LOCAL_POI, 1.0),
				PoiInterest::new(URBAN_POI, 0.45),
				PoiInterest::new(VEGETATION_POI, 0.45),
			]),
			Self::Brawler => PoiInterests::new([
				PoiInterest::new(SALOON_POI, 1.5),
				PoiInterest::new(URBAN_POI, 0.7),
				PoiInterest::new(LOCAL_POI, 0.55),
				PoiInterest::new(CHARACTER_POI, 1.0),
			]),
		};
		match (self, slot % 2) {
			(Self::Guard, 1) => interests.with_weight(LOCAL_POI, 1.3).with_weight(URBAN_POI, 0.7),
			(Self::Brawler, 1) => {
				interests.with_weight(SALOON_POI, 1.55).with_weight(URBAN_POI, 0.4)
			}
			_ => interests,
		}
	}

	pub const fn uses_long_range_routing(self) -> bool {
		!matches!(self, Self::Civilian)
	}

	pub const fn keep_tether_in_combat(self) -> bool {
		false
	}
}

impl FromMobNumber for CharacterBrains {
	fn from_num(num: f32) -> Self {
		Self::VALUES[index(num, 0xB2A1_65E5, Self::VALUES.len())]
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn guard_and_brawler_keep_local_urban_fallbacks() {
		let guard = CharacterBrains::Guard.interests();
		assert_eq!(guard.weight(URBAN_POI), Some(1.25));
		assert_eq!(guard.weight(LOCAL_POI), Some(0.9));
		let brawler = CharacterBrains::Brawler.interests();
		assert_eq!(brawler.weight(SALOON_POI), Some(1.5));
		assert_eq!(brawler.weight(URBAN_POI), Some(0.7));
		assert_eq!(brawler.weight(LOCAL_POI), Some(0.55));
		let local_guard = CharacterBrains::Guard.interests_for_slot(1);
		assert!(
			local_guard.weight(LOCAL_POI).unwrap_or(0.0)
				> local_guard.weight(URBAN_POI).unwrap_or(0.0)
		);
	}
}
