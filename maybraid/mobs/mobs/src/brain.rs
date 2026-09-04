//! Persistent host brains for generated mob families.

use bevy::prelude::Component;
use mob_characters::{CHARACTER_POI, LOCAL_POI, SALOON_POI, URBAN_POI, VEGETATION_POI};
use mob_intelligence::{MobAffiliations, MobRespawn, MobTravel};
use poi_intelligence::{PoiInterest, PoiInterests};
use threat_intelligence::{AffiliationStrength, Affiliations, ThreatGroupId};

use crate::MobKind;

const HERD_GROUP: ThreatGroupId = ThreatGroupId::group(30);
const PACK_GROUP: ThreatGroupId = ThreatGroupId::group(31);
const RAIDER_GROUP: ThreatGroupId = ThreatGroupId::group(32);
const GUARD_GROUP: ThreatGroupId = ThreatGroupId::group(33);
const PLEB_GROUP: ThreatGroupId = ThreatGroupId::group(34);
const BRAWLER_GROUP: ThreatGroupId = ThreatGroupId::group(35);

#[derive(Component, Clone, Debug)]
pub struct MobBrain {
	pub kind: MobKind,
	pub leash: f32,
	pub interests: PoiInterests,
	pub affiliations: MobAffiliations,
	pub respawn: MobRespawn,
	pub travel: Option<MobTravel>,
	pub journey: bool,
}

impl Default for MobBrain {
	fn default() -> Self {
		Self::for_kind(MobKind::Herd)
	}
}

impl MobBrain {
	pub fn for_kind(kind: MobKind) -> Self {
		let (leash, travel, journey) = match kind {
			MobKind::Herd => (24.0, Some(MobTravel::new(1.7)), true),
			MobKind::Pack => (16.0, Some(MobTravel::new(4.0)), true),
			MobKind::Raider => (20.0, Some(MobTravel::new(2.4)), true),
			MobKind::Guard => (12.0, None, false),
			MobKind::Pleb => (28.0, Some(MobTravel::new(1.0)), true),
			MobKind::Rambles => (22.0, Some(MobTravel::new(2.0)), true),
			MobKind::Brawler => (14.0, None, false),
		};
		Self {
			kind,
			leash,
			interests: interests(kind),
			affiliations: MobAffiliations::new(affiliations(kind)),
			respawn: MobRespawn::default(),
			travel,
			journey,
		}
	}
}

fn interests(kind: MobKind) -> PoiInterests {
	match kind {
		MobKind::Herd => PoiInterests::new([
			PoiInterest::new(VEGETATION_POI, 1.5),
			PoiInterest::new(LOCAL_POI, 0.35),
		]),
		MobKind::Pack => PoiInterests::new([
			PoiInterest::new(CHARACTER_POI, 1.6),
			PoiInterest::new(VEGETATION_POI, 0.25),
		]),
		MobKind::Raider => PoiInterests::new([
			PoiInterest::new(CHARACTER_POI, 1.25),
			PoiInterest::new(URBAN_POI, 1.0),
		]),
		MobKind::Guard => PoiInterests::new([
			PoiInterest::new(URBAN_POI, 1.4),
			PoiInterest::new(CHARACTER_POI, 0.6),
		]),
		MobKind::Pleb => {
			PoiInterests::new([PoiInterest::new(URBAN_POI, 1.2), PoiInterest::new(LOCAL_POI, 0.9)])
		}
		MobKind::Rambles => PoiInterests::new([
			PoiInterest::new(LOCAL_POI, 1.0),
			PoiInterest::new(URBAN_POI, 0.5),
			PoiInterest::new(VEGETATION_POI, 0.5),
		]),
		MobKind::Brawler => PoiInterests::new([
			PoiInterest::new(SALOON_POI, 1.6),
			PoiInterest::new(CHARACTER_POI, 1.0),
		]),
	}
}

fn affiliations(kind: MobKind) -> Affiliations {
	let own = group(kind);
	let mut affiliations = Affiliations::default();
	affiliations.join(own, AffiliationStrength::permanent(1.0));
	match kind {
		MobKind::Pack => {
			affiliations.antagonize(HERD_GROUP, AffiliationStrength::permanent(1.0));
			affiliations.mitigate(PACK_GROUP, AffiliationStrength::permanent(1.0));
		}
		MobKind::Raider => {
			affiliations.antagonize(PLEB_GROUP, AffiliationStrength::permanent(0.8));
			affiliations.antagonize(GUARD_GROUP, AffiliationStrength::permanent(0.7));
			affiliations.mitigate(RAIDER_GROUP, AffiliationStrength::permanent(1.0));
		}
		MobKind::Guard => {
			affiliations.antagonize(RAIDER_GROUP, AffiliationStrength::permanent(1.0));
			affiliations.mitigate(GUARD_GROUP, AffiliationStrength::permanent(1.0));
			affiliations.mitigate(PLEB_GROUP, AffiliationStrength::permanent(0.6));
		}
		MobKind::Brawler => {
			affiliations.antagonize(BRAWLER_GROUP, AffiliationStrength::permanent(1.0));
		}
		MobKind::Herd | MobKind::Pleb | MobKind::Rambles => {
			affiliations.antagonize(PACK_GROUP, AffiliationStrength::permanent(1.0));
			affiliations.mitigate(own, AffiliationStrength::permanent(1.0));
		}
	}
	affiliations
}

const fn group(kind: MobKind) -> ThreatGroupId {
	match kind {
		MobKind::Herd => HERD_GROUP,
		MobKind::Pack => PACK_GROUP,
		MobKind::Raider => RAIDER_GROUP,
		MobKind::Guard => GUARD_GROUP,
		MobKind::Pleb => PLEB_GROUP,
		MobKind::Rambles => ThreatGroupId::group(36),
		MobKind::Brawler => BRAWLER_GROUP,
	}
}
