//! Proto-mobs: a shared tether host plus personality-stamped members.
//!
//! This is not a LodScene host. The later mob brain should keep a roster like
//! this and fulfill High with the same [`Personality::install`] path.

use avian3d::prelude::{Collider, LockedAxes, RigidBody};
use bevy::prelude::*;
use damage::Health;
use firearm_intelligence::FirearmEngagement;
use lod_avian::PhysicsInteractionLayer;
use npc_intelligence::{NpcBody, NpcInstall, Personality};
use player::{spawn_npc, LocomotionCapsule, PlayerLook, CAPSULE_LENGTH, CAPSULE_RADIUS};
use poi_intelligence::{PoiInterest, PoiInterests};
use spotting_intelligence::{InterestLayers, SpotBounds, SpotSubject};
use std::f32::consts::TAU;
use threat_intelligence::{
	AffiliationStrength, Affiliations, ThreatGroupId, ThreatId, ThreatSubject,
};
use threat_management_intelligence::ThreatManagementIntelligence;

use crate::scene::{CAMP, FORAGE, GATE, PAD_EXTENT, PIT};

pub const PUBLIC: ThreatGroupId = ThreatGroupId::group(1);
pub const GUARD: ThreatGroupId = ThreatGroupId::group(2);
pub const HUNT: ThreatGroupId = ThreatGroupId::group(3);
pub const GRAZER: ThreatGroupId = ThreatGroupId::group(4);
pub const FFA: ThreatGroupId = ThreatGroupId::group(5);
pub const WILDLIFE: ThreatGroupId = ThreatGroupId::group(6);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MobKind {
	Herd,
	Occupy,
	Watch,
	Guard,
	Roam,
	Hunt,
	Ffa,
	Flock,
	Monk,
}

#[derive(Component, Clone, Copy, Debug)]
pub struct ProtoMob {
	pub kind: MobKind,
	pub leash: f32,
}

#[derive(Component, Clone, Copy, Debug)]
pub struct MobMember {
	pub mob: Entity,
}

#[derive(Component)]
pub struct PublicPresence;

#[derive(Clone, Copy, Debug)]
pub struct MemberSpec {
	pub personality: Personality,
	pub armed: bool,
	pub keep_tether_in_combat: bool,
	pub ffa: bool,
	pub discovery_radius: Option<f32>,
}

impl MemberSpec {
	const fn of(personality: Personality, armed: bool) -> Self {
		Self {
			personality,
			armed,
			keep_tether_in_combat: false,
			ffa: false,
			discovery_radius: None,
		}
	}

	const fn grazer() -> Self {
		Self::of(Personality::Grazer, false)
	}

	const fn civilian() -> Self {
		Self::of(Personality::Civilian, false)
	}

	const fn brawler() -> Self {
		Self::of(Personality::Brawler, true)
	}

	const fn guard() -> Self {
		Self { discovery_radius: Some(18.0), ..Self::brawler() }
	}

	const fn predator() -> Self {
		Self { keep_tether_in_combat: true, ..Self::of(Personality::Predator, true) }
	}

	const fn assassin() -> Self {
		Self { keep_tether_in_combat: true, ..Self::of(Personality::Assassin, true) }
	}

	const fn ffa() -> Self {
		Self { ffa: true, ..Self::brawler() }
	}
}

#[derive(Clone, Copy, Debug)]
pub struct MobRecipe {
	pub kind: MobKind,
	pub name: &'static str,
	pub at: Vec2,
	pub leash: f32,
	pub members: &'static [MemberSpec],
}

const HERD: &[MemberSpec] =
	&[MemberSpec::grazer(), MemberSpec::grazer(), MemberSpec::grazer(), MemberSpec::grazer()];
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
	&[MemberSpec::guard(), MemberSpec::guard(), MemberSpec::guard(), MemberSpec::guard()];
const GATE_GUARD: &[MemberSpec] = &[
	MemberSpec::guard(),
	MemberSpec::guard(),
	MemberSpec::guard(),
	MemberSpec::guard(),
	MemberSpec::brawler(),
	MemberSpec::brawler(),
];
const ROAM: &[MemberSpec] = &[
	MemberSpec::grazer(),
	MemberSpec::grazer(),
	MemberSpec::grazer(),
	MemberSpec::grazer(),
	MemberSpec::grazer(),
	MemberSpec::civilian(),
	MemberSpec::civilian(),
	MemberSpec::civilian(),
];
const HUNT_PACK: &[MemberSpec] = &[
	MemberSpec::predator(),
	MemberSpec::predator(),
	MemberSpec::predator(),
	MemberSpec::predator(),
	MemberSpec::assassin(),
	MemberSpec::assassin(),
];
const FFA_PIT: &[MemberSpec] = &[
	MemberSpec::ffa(),
	MemberSpec::ffa(),
	MemberSpec::ffa(),
	MemberSpec::ffa(),
	MemberSpec::ffa(),
	MemberSpec::ffa(),
	MemberSpec::ffa(),
	MemberSpec::ffa(),
];
const FLOCK: &[MemberSpec] = &[
	MemberSpec::grazer(),
	MemberSpec::grazer(),
	MemberSpec::grazer(),
	MemberSpec::grazer(),
	MemberSpec::grazer(),
];
const MONK: &[MemberSpec] = &[MemberSpec::grazer()];

pub fn recipes() -> [MobRecipe; 9] {
	[
		MobRecipe {
			kind: MobKind::Flock,
			name: "flock",
			at: Vec2::new(-14.0, 18.0),
			leash: 16.0,
			members: FLOCK,
		},
		MobRecipe {
			kind: MobKind::Herd,
			name: "herd",
			at: Vec2::new(20.0, -16.0),
			leash: 14.0,
			members: HERD,
		},
		MobRecipe {
			kind: MobKind::Occupy,
			name: "occupy",
			at: Vec2::new(-55.0, 10.0),
			leash: 20.0,
			members: OCCUPY,
		},
		MobRecipe {
			kind: MobKind::Watch,
			name: "watch",
			at: Vec2::new(32.0, 8.0),
			leash: 8.0,
			members: WATCH,
		},
		MobRecipe {
			kind: MobKind::Guard,
			name: "guard",
			at: Vec2::new(80.0, 20.0),
			leash: 10.0,
			members: GATE_GUARD,
		},
		MobRecipe {
			kind: MobKind::Roam,
			name: "roam",
			at: Vec2::new(-90.0, -90.0),
			leash: 22.0,
			members: ROAM,
		},
		MobRecipe {
			kind: MobKind::Hunt,
			name: "hunt",
			at: Vec2::new(20.0, 150.0),
			leash: 16.0,
			members: HUNT_PACK,
		},
		MobRecipe {
			kind: MobKind::Ffa,
			name: "ffa",
			at: Vec2::new(120.0, -40.0),
			leash: 12.0,
			members: FFA_PIT,
		},
		MobRecipe {
			kind: MobKind::Monk,
			name: "monk",
			at: Vec2::new(170.0, 170.0),
			leash: 28.0,
			members: MONK,
		},
	]
}

pub fn member_count() -> usize {
	recipes().iter().map(|recipe| recipe.members.len()).sum()
}

pub fn spawn_presence(
	commands: &mut Commands,
	meshes: &mut Assets<Mesh>,
	materials: &mut Assets<StandardMaterial>,
) -> Entity {
	let hull = LocomotionCapsule::HUMANOID;
	let at = Vec3::new(0.0, hull.spawn_height(), 0.0);
	let presence = commands
		.spawn((
			Name::new("public"),
			PublicPresence,
			Transform::from_translation(at),
			Visibility::default(),
			RigidBody::Kinematic,
			Collider::capsule(hull.radius, hull.length),
			PhysicsInteractionLayer::animated_layers(),
			LockedAxes::ROTATION_LOCKED,
			hull,
			Health::default(),
			SpotSubject::new(
				InterestLayers::CHARACTER,
				SpotBounds::capsule(hull.radius, hull.half_height()),
			),
		))
		.id();
	stamp_threat(commands, presence, public_affiliations(ThreatId(presence.to_bits())));
	commands.spawn((
		Name::new("PublicCapsule"),
		ChildOf(presence),
		Mesh3d(meshes.add(Capsule3d::new(CAPSULE_RADIUS, CAPSULE_LENGTH))),
		MeshMaterial3d(materials.add(Color::srgb(0.88, 0.92, 1.0))),
	));
	presence
}

pub fn spawn_mobs(
	commands: &mut Commands,
	meshes: &mut Assets<Mesh>,
	materials: &mut Assets<StandardMaterial>,
) {
	let capsule = meshes.add(Capsule3d::new(CAPSULE_RADIUS, CAPSULE_LENGTH));
	let colors = PersonalityColors::new(materials);
	let hull = LocomotionCapsule::HUMANOID;
	let body = NpcBody {
		agent_radius: hull.radius,
		feet_below_origin: hull.half_height(),
		eye_height: 1.45,
	};
	for recipe in recipes() {
		let host = commands
			.spawn((
				Name::new(recipe.name),
				ProtoMob { kind: recipe.kind, leash: recipe.leash },
				Transform::from_xyz(recipe.at.x, 0.0, recipe.at.y),
				Visibility::default(),
			))
			.id();
		for (index, spec) in recipe.members.iter().copied().enumerate() {
			let offset = member_offset(index, recipe.members.len(), (recipe.leash * 0.45).max(2.5));
			let at = Vec3::new(recipe.at.x + offset.x, hull.spawn_height(), recipe.at.y + offset.y);
			let npc = spawn_npc(commands, at, PlayerLook::default());
			spec.personality.install(
				commands,
				npc,
				NpcInstall {
					at,
					body,
					health: Health::default(),
					tether: Some(host),
					poi_interests: interests(recipe.kind),
					armed: spec.armed,
					keep_tether_in_combat: spec.keep_tether_in_combat.then_some(true),
					engagement: spec.ffa.then(FirearmEngagement::weapons_free),
					threat_override: spec.ffa.then(ThreatManagementIntelligence::ffa),
					discovery_radius: spec.discovery_radius,
					..NpcInstall::default()
				},
			);
			commands.entity(npc).insert((
				MobMember { mob: host },
				SpotSubject::new(
					InterestLayers::CHARACTER,
					SpotBounds::capsule(hull.radius, hull.half_height()),
				),
			));
			stamp_threat(commands, npc, affiliations(recipe.kind, ThreatId(npc.to_bits())));
			commands.spawn((
				Name::new("NpcCapsule"),
				ChildOf(npc),
				Mesh3d(capsule.clone()),
				MeshMaterial3d(colors.handle(spec.personality)),
			));
		}
	}
}

struct PersonalityColors {
	grazer: Handle<StandardMaterial>,
	civilian: Handle<StandardMaterial>,
	predator: Handle<StandardMaterial>,
	brawler: Handle<StandardMaterial>,
	assassin: Handle<StandardMaterial>,
}

impl PersonalityColors {
	fn new(materials: &mut Assets<StandardMaterial>) -> Self {
		Self {
			grazer: materials.add(Color::srgb(0.42, 0.82, 0.38)),
			civilian: materials.add(Color::srgb(0.86, 0.72, 0.4)),
			predator: materials.add(Color::srgb(0.95, 0.48, 0.14)),
			brawler: materials.add(Color::srgb(0.9, 0.22, 0.24)),
			assassin: materials.add(Color::srgb(0.58, 0.32, 0.88)),
		}
	}

	fn handle(&self, personality: Personality) -> Handle<StandardMaterial> {
		match personality {
			Personality::Grazer => self.grazer.clone(),
			Personality::Civilian => self.civilian.clone(),
			Personality::Predator => self.predator.clone(),
			Personality::Brawler => self.brawler.clone(),
			Personality::Assassin => self.assassin.clone(),
		}
	}
}

fn interests(kind: MobKind) -> PoiInterests {
	match kind {
		MobKind::Occupy | MobKind::Herd => {
			PoiInterests::new([PoiInterest::new(CAMP, 1.4), PoiInterest::new(FORAGE, 0.8)])
		}
		MobKind::Roam | MobKind::Monk | MobKind::Flock => {
			PoiInterests::new([PoiInterest::new(FORAGE, 1.3), PoiInterest::new(CAMP, 0.6)])
		}
		MobKind::Guard | MobKind::Watch => PoiInterests::one(GATE),
		MobKind::Hunt => PoiInterests::new([PoiInterest::new(FORAGE, 0.7)]),
		MobKind::Ffa => PoiInterests::one(PIT),
	}
}

fn affiliations(kind: MobKind, id: ThreatId) -> Affiliations {
	match kind {
		MobKind::Guard | MobKind::Watch => guard_affiliations(id),
		MobKind::Hunt => hunt_affiliations(id),
		MobKind::Ffa => ffa_affiliations(id),
		MobKind::Flock | MobKind::Monk => indifferent_affiliations(id),
		MobKind::Herd | MobKind::Occupy | MobKind::Roam => grazer_affiliations(id),
	}
}

pub fn public_affiliations(id: ThreatId) -> Affiliations {
	let mut affiliations = Affiliations::with_self(id);
	affiliations.join(PUBLIC, AffiliationStrength::permanent(1.0));
	affiliations
}

fn guard_affiliations(id: ThreatId) -> Affiliations {
	let mut affiliations = Affiliations::with_self(id);
	affiliations.join(GUARD, AffiliationStrength::permanent(1.0));
	affiliations.antagonize(PUBLIC, AffiliationStrength::permanent(1.0));
	affiliations.mitigate(GUARD, AffiliationStrength::permanent(1.0));
	affiliations
}

fn hunt_affiliations(id: ThreatId) -> Affiliations {
	let mut affiliations = Affiliations::with_self(id);
	affiliations.join(HUNT, AffiliationStrength::permanent(1.0));
	affiliations.antagonize(PUBLIC, AffiliationStrength::permanent(1.0));
	affiliations.antagonize(GRAZER, AffiliationStrength::permanent(1.0));
	affiliations.mitigate(HUNT, AffiliationStrength::permanent(1.0));
	affiliations
}

fn grazer_affiliations(id: ThreatId) -> Affiliations {
	let mut affiliations = Affiliations::with_self(id);
	affiliations.join(GRAZER, AffiliationStrength::permanent(1.0));
	affiliations.antagonize(PUBLIC, AffiliationStrength::permanent(1.0));
	affiliations.antagonize(HUNT, AffiliationStrength::permanent(1.0));
	affiliations.mitigate(GRAZER, AffiliationStrength::permanent(1.0));
	affiliations
}

fn indifferent_affiliations(id: ThreatId) -> Affiliations {
	let mut affiliations = Affiliations::with_self(id);
	affiliations.join(WILDLIFE, AffiliationStrength::permanent(1.0));
	affiliations.mitigate(WILDLIFE, AffiliationStrength::permanent(1.0));
	affiliations
}

fn ffa_affiliations(id: ThreatId) -> Affiliations {
	let mut affiliations = Affiliations::with_self(id);
	affiliations.join(FFA, AffiliationStrength::permanent(1.0));
	affiliations.antagonize(FFA, AffiliationStrength::permanent(1.0));
	affiliations
}

fn stamp_threat(commands: &mut Commands, entity: Entity, affiliations: Affiliations) {
	commands
		.entity(entity)
		.insert((ThreatSubject::new(ThreatId(entity.to_bits())), affiliations));
}

fn member_offset(index: usize, count: usize, radius: f32) -> Vec2 {
	let angle = index as f32 / count.max(1) as f32 * TAU;
	Vec2::new(angle.cos(), angle.sin()) * radius
}

pub fn clamp_to_pad(xz: Vec2) -> Vec2 {
	Vec2::new(
		xz.x.clamp(-PAD_EXTENT + 2.0, PAD_EXTENT - 2.0),
		xz.y.clamp(-PAD_EXTENT + 2.0, PAD_EXTENT - 2.0),
	)
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::scene::{HIGH_RING, SPOTTING_RING};

	#[test]
	fn recipes_cover_spotting_high_and_beyond() {
		let recipes = recipes();
		let dist = |recipe: &MobRecipe| recipe.at.length();
		let herd = recipes.iter().find(|recipe| recipe.kind == MobKind::Herd).unwrap();
		let watch = recipes.iter().find(|recipe| recipe.kind == MobKind::Watch).unwrap();
		let hunt = recipes.iter().find(|recipe| recipe.kind == MobKind::Hunt).unwrap();
		let monk = recipes.iter().find(|recipe| recipe.kind == MobKind::Monk).unwrap();
		assert!(dist(herd) < SPOTTING_RING * 0.5);
		assert!(dist(watch) < SPOTTING_RING);
		assert!(dist(hunt) > SPOTTING_RING && dist(hunt) < HIGH_RING);
		assert!(dist(monk) > HIGH_RING);
		assert!(dist(monk) < PAD_EXTENT * 2.0_f32.sqrt());
	}

	#[test]
	fn proto_mobs_fit_a_good_crowd() {
		assert_eq!(member_count(), 50);
		assert!(member_count() >= 32);
	}

	#[test]
	fn hunt_keeps_tether_in_combat() {
		assert!(HUNT_PACK.iter().all(|spec| spec.keep_tether_in_combat && spec.armed));
	}

	#[test]
	fn public_is_a_guard_threat_and_not_an_ffa_member() {
		let public = public_affiliations(ThreatId(1));
		let guard = guard_affiliations(ThreatId(2));
		let ffa = ffa_affiliations(ThreatId(3));
		assert!(guard.threat_weight(&public, 0.0) >= 0.2);
		assert_eq!(ffa.threat_weight(&public, 0.0), 0.0);
	}

	#[test]
	fn flock_and_monk_do_not_treat_public_as_a_threat() {
		let public = public_affiliations(ThreatId(1));
		let wildlife = indifferent_affiliations(ThreatId(4));
		let grazer = grazer_affiliations(ThreatId(5));
		assert_eq!(wildlife.threat_weight(&public, 0.0), 0.0);
		assert!(grazer.threat_weight(&public, 0.0) >= 0.2);
		let flock = recipes().into_iter().find(|recipe| recipe.kind == MobKind::Flock).unwrap();
		assert!(flock.at.length() < SPOTTING_RING * 0.5);
	}
}
