//! Packs: stationary occupy/watch plus traveling roam (loose) and hunt (tight).

use bevy::prelude::*;
use journeying_intelligence::JourneyingIntelligenceUser;
use mob_intelligence::{
	spawn_mob, MobIdAlloc, MobInstall, MobMemberBody, MobRespawn, MobSlot, MobTravel, RosterMember,
};
use npc_intelligence::{NpcBody, Personality};
use player::{spawn_npc, LocomotionCapsule, PlayerLook, CAPSULE_LENGTH, CAPSULE_RADIUS};
use poi_intelligence::{
	PoiIntelligenceUser, PoiInterest, PoiInterests, PoiKind, PoiKnowledge, PoiLearningPolicy,
	PoiVisitPolicy, PoiVisitState,
};
use spotting_intelligence::{InterestLayers, SpotBounds, SpotSubject};
use std::f32::consts::TAU;
use threat_intelligence::{AffiliationStrength, Affiliations, ThreatGroupId};

use crate::scene::{waypoint_xz, CAMP, FORAGE, GATE, JOURNEY_TILE, WAYPOINT};

const WILDLIFE: ThreatGroupId = ThreatGroupId::group(6);

#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub enum PackKind {
	Occupy,
	Watch,
	Roam,
	Hunt,
}

#[derive(Clone, Copy, Debug)]
pub struct PackRecipe {
	pub kind: PackKind,
	pub name: &'static str,
	pub at: Vec2,
	pub leash: f32,
	pub travel: f32,
	pub journey_linger: f32,
	pub members: &'static [MemberSpec],
}

impl PackRecipe {
	pub fn traveling(&self) -> bool {
		self.travel > 0.0
	}
}

#[derive(Clone, Copy, Debug)]
pub struct MemberSpec {
	pub personality: Personality,
	pub armed: bool,
	pub keep_tether_in_combat: bool,
}

impl MemberSpec {
	const fn grazer() -> Self {
		Self { personality: Personality::Grazer, armed: false, keep_tether_in_combat: false }
	}

	const fn civilian() -> Self {
		Self { personality: Personality::Civilian, armed: false, keep_tether_in_combat: false }
	}

	const fn brawler() -> Self {
		Self { personality: Personality::Brawler, armed: true, keep_tether_in_combat: false }
	}

	const fn predator() -> Self {
		Self { personality: Personality::Predator, armed: true, keep_tether_in_combat: true }
	}

	const fn assassin() -> Self {
		Self { personality: Personality::Assassin, armed: true, keep_tether_in_combat: true }
	}
}

const OCCUPY: &[MemberSpec] = &[
	MemberSpec::grazer(),
	MemberSpec::grazer(),
	MemberSpec::grazer(),
	MemberSpec::grazer(),
	MemberSpec::civilian(),
	MemberSpec::civilian(),
	MemberSpec::civilian(),
	MemberSpec::civilian(),
];
const WATCH: &[MemberSpec] =
	&[MemberSpec::brawler(), MemberSpec::brawler(), MemberSpec::brawler(), MemberSpec::brawler()];
const ROAM: &[MemberSpec] = &[
	MemberSpec::grazer(),
	MemberSpec::grazer(),
	MemberSpec::grazer(),
	MemberSpec::grazer(),
	MemberSpec::civilian(),
	MemberSpec::civilian(),
];
const HUNT: &[MemberSpec] =
	&[MemberSpec::predator(), MemberSpec::predator(), MemberSpec::assassin()];

pub fn recipes() -> [PackRecipe; 4] {
	[
		PackRecipe {
			kind: PackKind::Occupy,
			name: "occupy",
			at: Vec2::new(-110.0, 105.0),
			leash: 20.0,
			travel: 0.0,
			journey_linger: 0.0,
			members: OCCUPY,
		},
		PackRecipe {
			kind: PackKind::Watch,
			name: "watch",
			at: Vec2::new(118.0, 95.0),
			leash: 10.0,
			travel: 0.0,
			journey_linger: 0.0,
			members: WATCH,
		},
		PackRecipe {
			kind: PackKind::Roam,
			name: "roam",
			at: Vec2::new(-45.0, -55.0),
			leash: 24.0,
			travel: 1.7,
			journey_linger: 4.5,
			members: ROAM,
		},
		PackRecipe {
			kind: PackKind::Hunt,
			name: "hunt",
			at: Vec2::new(70.0, -115.0),
			leash: 16.0,
			travel: 5.4,
			journey_linger: 1.2,
			members: HUNT,
		},
	]
}

pub fn interests(kind: PackKind) -> PoiInterests {
	match kind {
		PackKind::Occupy => {
			PoiInterests::new([PoiInterest::new(CAMP, 1.4), PoiInterest::new(FORAGE, 0.8)])
		}
		PackKind::Watch => PoiInterests::one(GATE),
		PackKind::Roam => {
			PoiInterests::new([PoiInterest::new(FORAGE, 1.3), PoiInterest::new(CAMP, 0.5)])
		}
		PackKind::Hunt => PoiInterests::default(),
	}
}

/// Local destinations: occupy/watch clusters plus forage along the pad for roam.
pub fn poi_placements() -> Vec<(PoiKind, Vec2)> {
	let mut placements = Vec::new();
	for recipe in recipes() {
		let radius = recipe.leash * 0.7;
		let (primary, primary_n, secondary, secondary_n) = match recipe.kind {
			PackKind::Occupy => (CAMP, 10, Some(FORAGE), 4),
			PackKind::Watch => (GATE, 5, None, 0),
			PackKind::Roam => (FORAGE, 4, Some(CAMP), 2),
			PackKind::Hunt => continue,
		};
		let total = primary_n + secondary_n;
		for index in 0..total {
			let kind = if index < primary_n { primary } else { secondary.unwrap_or(primary) };
			placements.push((kind, disk_point(recipe.at, radius, index, total)));
		}
	}
	placements.extend(forage_along_waypoints());
	placements
}

fn forage_along_waypoints() -> Vec<(PoiKind, Vec2)> {
	let mut placements = Vec::new();
	for (index, at) in waypoint_xz().into_iter().enumerate() {
		let offset = disk_point(Vec2::ZERO, 16.0, index, 10);
		placements.push((FORAGE, at + offset));
		if index % 3 == 0 {
			placements.push((CAMP, at - offset * 0.6));
		}
	}
	placements
}

#[derive(Resource)]
struct NpcVisuals {
	capsule: Handle<Mesh>,
	colors: PersonalityColors,
}

pub fn spawn_packs(
	commands: &mut Commands,
	meshes: &mut Assets<Mesh>,
	materials: &mut Assets<StandardMaterial>,
	ids: &mut MobIdAlloc,
) {
	let visuals = NpcVisuals {
		capsule: meshes.add(Capsule3d::new(CAPSULE_RADIUS, CAPSULE_LENGTH)),
		colors: PersonalityColors::new(materials),
	};
	let marker = meshes.add(Sphere::new(1.8));
	let hull = LocomotionCapsule::HUMANOID;
	for recipe in recipes() {
		let id = ids.allocate();
		let members: Vec<_> = recipe
			.members
			.iter()
			.copied()
			.enumerate()
			.map(|(index, spec)| {
				let offset =
					member_offset(index, recipe.members.len(), (recipe.leash * 0.4).max(2.0));
				let at =
					Vec3::new(recipe.at.x + offset.x, hull.spawn_height(), recipe.at.y + offset.y);
				RosterMember::new(spec.personality, at)
					.with_armed(spec.armed)
					.with_keep_tether_in_combat(spec.keep_tether_in_combat.then_some(true))
			})
			.collect();
		let mut install = MobInstall::new(id, recipe.leash, members)
			.with_respawn(MobRespawn::never())
			.with_affiliations(mob_intelligence::MobAffiliations::new(wildlife_pack()))
			.with_interests(interests(recipe.kind));
		if recipe.traveling() {
			install = install.with_travel(MobTravel::new(recipe.travel));
		}
		let host = spawn_mob(commands, Transform::from_xyz(recipe.at.x, 0.0, recipe.at.y), install);
		commands.entity(host).insert((Name::new(recipe.name), recipe.kind));
		if recipe.traveling() {
			stamp_journey(commands, host, id.0, recipe.journey_linger);
		}
		commands.spawn((
			Name::new("TetherMarker"),
			ChildOf(host),
			Mesh3d(marker.clone()),
			MeshMaterial3d(visuals.colors.host(recipe.kind)),
			Transform::from_xyz(0.0, 2.2, 0.0),
		));
		for (index, spec) in recipe.members.iter().copied().enumerate() {
			let offset = member_offset(index, recipe.members.len(), (recipe.leash * 0.4).max(2.0));
			let at = Vec3::new(recipe.at.x + offset.x, hull.spawn_height(), recipe.at.y + offset.y);
			spawn_member(commands, &visuals, id, index as u16, spec, at);
		}
	}
	commands.insert_resource(visuals);
}

fn stamp_journey(commands: &mut Commands, host: Entity, seed: u64, linger_secs: f32) {
	let mut journey = JourneyingIntelligenceUser::new(seed);
	journey.tile_size = JOURNEY_TILE;
	journey.min_tile_distance = 1;
	journey.max_tile_distance = 5;
	journey.tile_probes = 16;
	journey.selection_interval = 0.2;
	journey.linger_secs = linger_secs;
	journey.empty_tile_retry_secs = 4.0;
	journey.visit_policy = PoiVisitPolicy::Weighted {
		novelty_weight: 2.0,
		revisit_cooldown_secs: 10.0,
		repeat_weight: 0.8,
	};
	let mut learner = PoiIntelligenceUser::new(PoiInterests::one(WAYPOINT));
	learner.policy = PoiLearningPolicy {
		global_scan_interval: 0.4,
		learning_rate_per_second: 16.0,
		..learner.policy
	};
	commands.entity(host).insert((
		journey,
		learner,
		PoiKnowledge::default(),
		PoiVisitState::default(),
	));
}

fn spawn_member(
	commands: &mut Commands,
	visuals: &NpcVisuals,
	id: mob_intelligence::MobId,
	slot: u16,
	spec: MemberSpec,
	at: Vec3,
) {
	let hull = LocomotionCapsule::HUMANOID;
	let npc = spawn_npc(commands, at, PlayerLook::default());
	commands.entity(npc).insert((
		MobSlot(slot),
		id,
		MobMemberBody(NpcBody {
			agent_radius: hull.radius,
			feet_below_origin: hull.half_height(),
			eye_height: 1.45,
		}),
		SpotSubject::new(
			InterestLayers::CHARACTER,
			SpotBounds::capsule(hull.radius, hull.half_height()),
		),
	));
	commands.spawn((
		Name::new("NpcCapsule"),
		ChildOf(npc),
		Mesh3d(visuals.capsule.clone()),
		MeshMaterial3d(visuals.colors.handle(spec.personality)),
	));
}

fn wildlife_pack() -> Affiliations {
	let mut affiliations = Affiliations::default();
	affiliations.join(WILDLIFE, AffiliationStrength::permanent(1.0));
	affiliations.mitigate(WILDLIFE, AffiliationStrength::permanent(1.0));
	affiliations
}

fn member_offset(index: usize, count: usize, radius: f32) -> Vec2 {
	let angle = index as f32 / count.max(1) as f32 * TAU;
	Vec2::new(angle.cos(), angle.sin()) * radius
}

fn disk_point(center: Vec2, radius: f32, index: usize, count: usize) -> Vec2 {
	if count <= 1 {
		return center;
	}
	const GOLDEN: f32 = 2.399_963;
	let t = (index as f32 + 0.5) / count as f32;
	let r = radius * t.sqrt();
	let angle = index as f32 * GOLDEN;
	center + Vec2::new(angle.cos(), angle.sin()) * r
}

struct PersonalityColors {
	grazer: Handle<StandardMaterial>,
	civilian: Handle<StandardMaterial>,
	brawler: Handle<StandardMaterial>,
	predator: Handle<StandardMaterial>,
	assassin: Handle<StandardMaterial>,
	occupy_host: Handle<StandardMaterial>,
	watch_host: Handle<StandardMaterial>,
	roam_host: Handle<StandardMaterial>,
	hunt_host: Handle<StandardMaterial>,
}

impl PersonalityColors {
	fn new(materials: &mut Assets<StandardMaterial>) -> Self {
		Self {
			grazer: materials.add(Color::srgb(0.42, 0.82, 0.38)),
			civilian: materials.add(Color::srgb(0.86, 0.72, 0.4)),
			brawler: materials.add(Color::srgb(0.9, 0.22, 0.24)),
			predator: materials.add(Color::srgb(0.95, 0.48, 0.14)),
			assassin: materials.add(Color::srgb(0.58, 0.32, 0.88)),
			occupy_host: materials.add(Color::srgb(0.55, 0.95, 0.4)),
			watch_host: materials.add(Color::srgb(0.95, 0.62, 0.2)),
			roam_host: materials.add(Color::srgb(0.35, 0.7, 1.0)),
			hunt_host: materials.add(Color::srgb(1.0, 0.28, 0.22)),
		}
	}

	fn handle(&self, personality: Personality) -> Handle<StandardMaterial> {
		match personality {
			Personality::Grazer => self.grazer.clone(),
			Personality::Civilian => self.civilian.clone(),
			Personality::Brawler => self.brawler.clone(),
			Personality::Predator => self.predator.clone(),
			Personality::Assassin => self.assassin.clone(),
		}
	}

	fn host(&self, kind: PackKind) -> Handle<StandardMaterial> {
		match kind {
			PackKind::Occupy => self.occupy_host.clone(),
			PackKind::Watch => self.watch_host.clone(),
			PackKind::Roam => self.roam_host.clone(),
			PackKind::Hunt => self.hunt_host.clone(),
		}
	}
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
	use super::*;

	#[test]
	fn packs_mix_stationary_loose_and_tight() {
		let recipes = recipes();
		assert_eq!(recipes.len(), 4);
		let occupy = recipes.iter().find(|recipe| recipe.kind == PackKind::Occupy).unwrap();
		let watch = recipes.iter().find(|recipe| recipe.kind == PackKind::Watch).unwrap();
		let roam = recipes.iter().find(|recipe| recipe.kind == PackKind::Roam).unwrap();
		let hunt = recipes.iter().find(|recipe| recipe.kind == PackKind::Hunt).unwrap();
		assert!(!occupy.traveling());
		assert!(!watch.traveling());
		assert!(roam.traveling());
		assert!(hunt.traveling());
		assert!(roam.travel < hunt.travel);
		assert!(OCCUPY.iter().any(|spec| spec.personality == Personality::Civilian));
		assert!(WATCH.iter().all(|spec| spec.personality == Personality::Brawler));
		assert!(HUNT.iter().all(|spec| spec.keep_tether_in_combat && spec.armed));
		assert_eq!(recipes.iter().map(|recipe| recipe.members.len()).sum::<usize>(), 21);
	}

	#[test]
	fn hunt_members_have_no_local_interests() {
		assert!(interests(PackKind::Hunt).is_empty());
		assert!(interests(PackKind::Roam).iter().any(|interest| interest.kind == FORAGE));
		assert!(interests(PackKind::Occupy).iter().any(|interest| interest.kind == CAMP));
		assert!(interests(PackKind::Watch).iter().any(|interest| interest.kind == GATE));
	}

	#[test]
	fn local_pois_cover_stationary_and_roam() {
		let placements = poi_placements();
		assert!(placements
			.iter()
			.any(|(kind, at)| *kind == CAMP && at.distance(Vec2::new(-110.0, 105.0)) < 20.0));
		assert!(placements
			.iter()
			.any(|(kind, at)| *kind == GATE && at.distance(Vec2::new(118.0, 95.0)) < 10.0));
		assert!(placements.iter().filter(|(kind, _)| *kind == FORAGE).count() >= 10);
		assert!(!placements.is_empty());
	}
}
