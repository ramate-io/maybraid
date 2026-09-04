//! Packs: stationary occupy/watch plus traveling roam (loose) and hunt (tight).

use bevy::prelude::*;
use journeying_intelligence::JourneyingIntelligenceUser;
use mob_intelligence::{
	spawn_mob, Mob, MobIdAlloc, MobInstall, MobMemberBody, MobRespawn, MobSlot, MobTravel,
	RosterMember,
};
use npc_intelligence::{NpcBody, Personality};
use player::{spawn_npc, LocomotionCapsule, PlayerLook, CAPSULE_LENGTH, CAPSULE_RADIUS};
use poi_intelligence::{
	PoiGoal, PoiId, PoiIntelligenceUser, PoiInterest, PoiInterests, PoiKind, PoiKnowledge,
	PoiLearningPolicy, PoiVisitPolicy, PoiVisitState,
};
use spotting_intelligence::{InterestLayers, SpotBounds, SpotSubject};
use std::f32::consts::TAU;
use threat_intelligence::{AffiliationStrength, Affiliations, ThreatGroupId};

use crate::scene::{waypoint_xz, CAMP, FORAGE, GATE, JOURNEY_TILE, WAYPOINT};

const GRAZER_GROUP: ThreatGroupId = ThreatGroupId::group(4);
const HUNT_GROUP: ThreatGroupId = ThreatGroupId::group(3);
const WILDLIFE: ThreatGroupId = ThreatGroupId::group(6);
const PREY: PoiKind = PoiKind::new("mob-brain/prey");
const PREY_POI: PoiId = PoiId(10_000);
const HUNT_ARRIVAL: f32 = 12.0;
const HUNT_LOCK_SECS: f32 = 6.0;

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
	pub journey: bool,
	pub journey_linger: f32,
	pub members: &'static [MemberSpec],
}

impl PackRecipe {
	pub fn traveling(&self) -> bool {
		self.travel > 0.0
	}

	pub fn journeys(&self) -> bool {
		self.journey
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
		Self { personality: Personality::Predator, armed: true, keep_tether_in_combat: false }
	}

	const fn assassin() -> Self {
		Self { personality: Personality::Assassin, armed: true, keep_tether_in_combat: false }
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
	MemberSpec::grazer(),
	MemberSpec::grazer(),
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
			journey: false,
			journey_linger: 0.0,
			members: OCCUPY,
		},
		PackRecipe {
			kind: PackKind::Watch,
			name: "watch",
			at: Vec2::new(118.0, 95.0),
			leash: 10.0,
			travel: 0.0,
			journey: false,
			journey_linger: 0.0,
			members: WATCH,
		},
		PackRecipe {
			kind: PackKind::Roam,
			name: "herd",
			at: Vec2::new(-55.0, -35.0),
			leash: 24.0,
			travel: 1.7,
			journey: true,
			journey_linger: 4.5,
			members: ROAM,
		},
		PackRecipe {
			kind: PackKind::Hunt,
			name: "hunt",
			at: Vec2::new(45.0, -50.0),
			leash: 16.0,
			travel: 4.0,
			journey: false,
			journey_linger: HUNT_LOCK_SECS,
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
			.with_affiliations(mob_intelligence::MobAffiliations::new(pack_affiliations(
				recipe.kind,
			)))
			.with_interests(interests(recipe.kind));
		if recipe.traveling() {
			install = install.with_travel(MobTravel::new(recipe.travel));
		}
		let host = spawn_mob(commands, Transform::from_xyz(recipe.at.x, 0.0, recipe.at.y), install);
		commands.entity(host).insert((Name::new(recipe.name), recipe.kind));
		if recipe.journeys() {
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

/// Hunt host travels onto the herd. Arrival locks member tethers onto that host.
pub fn hunt_tracks_herd(
	time: Res<Time>,
	mut commands: Commands,
	hosts: Query<(Entity, &PackKind, &GlobalTransform), With<Mob>>,
	mut goals: Query<&mut PoiGoal>,
) {
	let Some((prey, prey_at)) = hosts.iter().find_map(|(entity, kind, transform)| {
		(*kind == PackKind::Roam).then_some((entity, transform.translation()))
	}) else {
		return;
	};
	let now = time.elapsed_secs();
	let at = Vec3::new(prey_at.x, 0.0, prey_at.z);
	for (hunt, kind, _) in &hosts {
		if *kind != PackKind::Hunt {
			continue;
		}
		if let Ok(mut goal) = goals.get_mut(hunt) {
			goal.target = PREY_POI;
			goal.kind = PREY;
			goal.poi_entity = Some(prey);
			goal.location.point = at;
			goal.location.radius = HUNT_ARRIVAL;
			goal.linger_secs = HUNT_LOCK_SECS;
			continue;
		}
		commands.entity(hunt).insert(PoiGoal::new(
			1,
			PREY_POI,
			Some(prey),
			PREY,
			at,
			HUNT_ARRIVAL,
			now,
			HUNT_LOCK_SECS,
		));
	}
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

fn pack_affiliations(kind: PackKind) -> Affiliations {
	match kind {
		PackKind::Hunt => hunt_pack(),
		PackKind::Roam => grazer_pack(),
		PackKind::Occupy | PackKind::Watch => wildlife_pack(),
	}
}

fn hunt_pack() -> Affiliations {
	let mut affiliations = Affiliations::default();
	affiliations.join(HUNT_GROUP, AffiliationStrength::permanent(1.0));
	affiliations.antagonize(GRAZER_GROUP, AffiliationStrength::permanent(1.0));
	affiliations.mitigate(HUNT_GROUP, AffiliationStrength::permanent(1.0));
	affiliations
}

fn grazer_pack() -> Affiliations {
	let mut affiliations = Affiliations::default();
	affiliations.join(GRAZER_GROUP, AffiliationStrength::permanent(1.0));
	affiliations.antagonize(HUNT_GROUP, AffiliationStrength::permanent(1.0));
	affiliations.mitigate(GRAZER_GROUP, AffiliationStrength::permanent(1.0));
	affiliations
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
		assert!(roam.journeys());
		assert!(hunt.traveling());
		assert!(!hunt.journeys());
		assert!(roam.travel < hunt.travel);
		assert!(ROAM.iter().all(|spec| spec.personality == Personality::Grazer));
		assert!(WATCH.iter().all(|spec| spec.personality == Personality::Brawler));
		assert!(HUNT.iter().all(|spec| spec.armed && !spec.keep_tether_in_combat));
		assert_eq!(recipes.iter().map(|recipe| recipe.members.len()).sum::<usize>(), 21);
		assert!(
			recipes
				.iter()
				.find(|recipe| recipe.kind == PackKind::Roam)
				.unwrap()
				.at
				.distance(recipes.iter().find(|recipe| recipe.kind == PackKind::Hunt).unwrap().at)
				> 40.0
		);
	}

	#[test]
	fn hunt_antagonizes_the_herd() {
		assert!(pack_affiliations(PackKind::Hunt).known_antagonists.contains_key(&GRAZER_GROUP));
		assert!(pack_affiliations(PackKind::Roam).known_antagonists.contains_key(&HUNT_GROUP));
		assert!(pack_affiliations(PackKind::Occupy).known_antagonists.is_empty());
		assert!(pack_affiliations(PackKind::Watch).known_antagonists.is_empty());
	}

	#[test]
	fn hunt_lock_linger_matches_the_chase_goal() {
		let hunt = recipes().into_iter().find(|recipe| recipe.kind == PackKind::Hunt).unwrap();
		assert!((hunt.journey_linger - HUNT_LOCK_SECS).abs() < 1e-4);
		assert!(!hunt.journeys());
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
