//! Traveling packs: the host is the tether, journeying relocates it.

use bevy::prelude::*;
use journeying_intelligence::JourneyingIntelligenceUser;
use mob_intelligence::{
	spawn_mob, MobIdAlloc, MobInstall, MobMemberBody, MobRespawn, MobSlot, MobTravel, RosterMember,
};
use npc_intelligence::{NpcBody, Personality};
use player::{spawn_npc, LocomotionCapsule, PlayerLook, CAPSULE_LENGTH, CAPSULE_RADIUS};
use poi_intelligence::{
	PoiIntelligenceUser, PoiInterests, PoiKnowledge, PoiLearningPolicy, PoiVisitPolicy,
	PoiVisitState,
};
use spotting_intelligence::{InterestLayers, SpotBounds, SpotSubject};
use std::f32::consts::TAU;
use threat_intelligence::{AffiliationStrength, Affiliations, ThreatGroupId};

use crate::scene::{JOURNEY_TILE, WAYPOINT};

const WILDLIFE: ThreatGroupId = ThreatGroupId::group(6);

#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub enum PackKind {
	Herd,
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
	pub members: &'static [MemberSpec],
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

	const fn predator() -> Self {
		Self { personality: Personality::Predator, armed: true, keep_tether_in_combat: true }
	}

	const fn assassin() -> Self {
		Self { personality: Personality::Assassin, armed: true, keep_tether_in_combat: true }
	}
}

const HERD: &[MemberSpec] = &[
	MemberSpec::grazer(),
	MemberSpec::grazer(),
	MemberSpec::grazer(),
	MemberSpec::grazer(),
	MemberSpec::grazer(),
];
const ROAM: &[MemberSpec] =
	&[MemberSpec::grazer(), MemberSpec::grazer(), MemberSpec::grazer(), MemberSpec::grazer()];
const HUNT: &[MemberSpec] =
	&[MemberSpec::predator(), MemberSpec::predator(), MemberSpec::assassin()];

pub fn recipes() -> [PackRecipe; 3] {
	[
		PackRecipe {
			kind: PackKind::Herd,
			name: "herd",
			at: Vec2::new(-70.0, 30.0),
			leash: 12.0,
			travel: 2.4,
			members: HERD,
		},
		PackRecipe {
			kind: PackKind::Roam,
			name: "roam",
			at: Vec2::new(50.0, -55.0),
			leash: 18.0,
			travel: 4.2,
			members: ROAM,
		},
		PackRecipe {
			kind: PackKind::Hunt,
			name: "hunt",
			at: Vec2::new(15.0, 85.0),
			leash: 16.0,
			travel: 6.0,
			members: HUNT,
		},
	]
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
		let host = spawn_mob(
			commands,
			Transform::from_xyz(recipe.at.x, 0.0, recipe.at.y),
			MobInstall::new(id, recipe.leash, members)
				.with_travel(MobTravel::new(recipe.travel))
				.with_respawn(MobRespawn::never())
				.with_affiliations(mob_intelligence::MobAffiliations::new(wildlife_pack())),
		);
		commands.entity(host).insert((Name::new(recipe.name), recipe.kind));
		stamp_journey(commands, host, id.0);
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

fn stamp_journey(commands: &mut Commands, host: Entity, seed: u64) {
	let mut journey = JourneyingIntelligenceUser::new(seed);
	journey.tile_size = JOURNEY_TILE;
	journey.min_tile_distance = 1;
	journey.max_tile_distance = 5;
	journey.tile_probes = 16;
	journey.selection_interval = 0.2;
	journey.linger_secs = 2.5;
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

/// Members follow the host; they do not pick the same long-range waypoints.
pub fn quiet_member_meander(
	mut members: Query<&mut PoiIntelligenceUser, Added<mob_intelligence::MemberOf>>,
) {
	for mut learner in &mut members {
		learner.interests = PoiInterests::default();
	}
}

fn member_offset(index: usize, count: usize, radius: f32) -> Vec2 {
	let angle = index as f32 / count.max(1) as f32 * TAU;
	Vec2::new(angle.cos(), angle.sin()) * radius
}

struct PersonalityColors {
	grazer: Handle<StandardMaterial>,
	predator: Handle<StandardMaterial>,
	assassin: Handle<StandardMaterial>,
	herd_host: Handle<StandardMaterial>,
	roam_host: Handle<StandardMaterial>,
	hunt_host: Handle<StandardMaterial>,
}

impl PersonalityColors {
	fn new(materials: &mut Assets<StandardMaterial>) -> Self {
		Self {
			grazer: materials.add(Color::srgb(0.42, 0.82, 0.38)),
			predator: materials.add(Color::srgb(0.95, 0.48, 0.14)),
			assassin: materials.add(Color::srgb(0.58, 0.32, 0.88)),
			herd_host: materials.add(Color::srgb(0.2, 0.95, 0.55)),
			roam_host: materials.add(Color::srgb(0.35, 0.7, 1.0)),
			hunt_host: materials.add(Color::srgb(1.0, 0.28, 0.22)),
		}
	}

	fn handle(&self, personality: Personality) -> Handle<StandardMaterial> {
		match personality {
			Personality::Grazer | Personality::Civilian | Personality::Brawler => {
				self.grazer.clone()
			}
			Personality::Predator => self.predator.clone(),
			Personality::Assassin => self.assassin.clone(),
		}
	}

	fn host(&self, kind: PackKind) -> Handle<StandardMaterial> {
		match kind {
			PackKind::Herd => self.herd_host.clone(),
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
	fn three_packs_travel_at_different_speeds() {
		let recipes = recipes();
		assert_eq!(recipes.len(), 3);
		let herd = recipes.iter().find(|recipe| recipe.kind == PackKind::Herd).unwrap();
		let roam = recipes.iter().find(|recipe| recipe.kind == PackKind::Roam).unwrap();
		let hunt = recipes.iter().find(|recipe| recipe.kind == PackKind::Hunt).unwrap();
		assert!(herd.travel < roam.travel);
		assert!(roam.travel < hunt.travel);
		assert!(HUNT.iter().all(|spec| spec.keep_tether_in_combat && spec.armed));
		assert_eq!(recipes.iter().map(|recipe| recipe.members.len()).sum::<usize>(), 12);
	}
}
