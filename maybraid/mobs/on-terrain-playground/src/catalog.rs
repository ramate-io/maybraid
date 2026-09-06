//! Short authored casts. World generation can put a dozen members in a cell;
//! this playground keeps the roster tiny so host Y and plant follow are readable.

use std::sync::Arc;

use bevy::prelude::*;
use maybraid_mobs::{MobKind, MobRosterRecipe, MobScene};
use mob_characters::{CharacterInventory, CharacterSpecies, MobCharacter};

/// Stable seed for `MobScene::of_kind`. Count is truncated after generation.
pub const PLAYGROUND_NUM: f32 = 4.0;
pub const HERD_MEMBERS: usize = 6;
pub const PACK_MEMBERS: usize = 4;
/// Hars/Ylter casts stay tiny so pitch probes are easy to read.
pub const SPECIES_MEMBERS: usize = 2;
/// Whole 4×4 patch stays High so plants do not cull while the host journeys.
pub const HIGH_RADIUS: f32 = 2_000.0;
/// Same tile as the mob-brain pad so a 90 m forage ring is a neighboring cell.
pub const JOURNEY_TILE: f32 = 48.0;
/// Grazer personality is 24 m; this patch needs room to meander without a yank.
pub const HERD_LEASH: f32 = 80.0;
pub const PACK_LEASH: f32 = 48.0;

/// Which authored hosts to present.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PlaygroundCast {
	#[default]
	Herd,
	Pack,
	Both,
	Hars,
	Ylter,
	HarsYlter,
}

/// One host in a cast: family brains plus an optional species override.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CastPlacement {
	pub kind: MobKind,
	pub species: Option<CharacterSpecies>,
	pub offset: Vec2,
}

const HERD_PLACEMENT: [CastPlacement; 1] = [CastPlacement {
	kind: MobKind::Herd,
	species: None,
	offset: Vec2::new(-40.0, -20.0),
}];
const PACK_PLACEMENT: [CastPlacement; 1] = [CastPlacement {
	kind: MobKind::Pack,
	species: None,
	offset: Vec2::new(36.0, -16.0),
}];
const BOTH_PLACEMENTS: [CastPlacement; 2] = [
	CastPlacement { kind: MobKind::Herd, species: None, offset: Vec2::new(-40.0, -20.0) },
	CastPlacement { kind: MobKind::Pack, species: None, offset: Vec2::new(48.0, 24.0) },
];
const HARS_PLACEMENT: [CastPlacement; 1] = [CastPlacement {
	kind: MobKind::Herd,
	species: Some(CharacterSpecies::Hars),
	offset: Vec2::new(-40.0, -20.0),
}];
const YLTER_PLACEMENT: [CastPlacement; 1] = [CastPlacement {
	kind: MobKind::Herd,
	species: Some(CharacterSpecies::Ylter),
	offset: Vec2::new(36.0, -16.0),
}];
const HARS_YLTER_PLACEMENTS: [CastPlacement; 2] = [
	CastPlacement {
		kind: MobKind::Herd,
		species: Some(CharacterSpecies::Hars),
		offset: Vec2::new(-40.0, -20.0),
	},
	CastPlacement {
		kind: MobKind::Herd,
		species: Some(CharacterSpecies::Ylter),
		offset: Vec2::new(36.0, -16.0),
	},
];

impl PlaygroundCast {
	pub fn label(self) -> &'static str {
		match self {
			Self::Herd => "herd",
			Self::Pack => "pack",
			Self::Both => "herd+pack",
			Self::Hars => "hars",
			Self::Ylter => "ylter",
			Self::HarsYlter => "hars+ylter",
		}
	}

	/// Hosts and XZ offsets from the terrain-patch center.
	pub fn placements(self) -> &'static [CastPlacement] {
		match self {
			Self::Herd => &HERD_PLACEMENT,
			Self::Pack => &PACK_PLACEMENT,
			Self::Both => &BOTH_PLACEMENTS,
			Self::Hars => &HARS_PLACEMENT,
			Self::Ylter => &YLTER_PLACEMENT,
			Self::HarsYlter => &HARS_YLTER_PLACEMENTS,
		}
	}
}

pub fn playground_leash(kind: MobKind) -> f32 {
	match kind {
		MobKind::Pack => PACK_LEASH,
		_ => HERD_LEASH,
	}
}

pub fn scene_for(kind: MobKind) -> MobScene {
	scene_for_placement(CastPlacement { kind, species: None, offset: Vec2::ZERO })
}

pub fn scene_for_placement(placement: CastPlacement) -> MobScene {
	let mut scene = scene_for_kind(placement.kind);
	if let Some(species) = placement.species {
		force_species(&mut scene, species);
	}
	scene.mob.roster.members.truncate(member_cap(placement));
	scene
}

fn scene_for_kind(kind: MobKind) -> MobScene {
	let mut scene = MobScene::of_kind(kind, PLAYGROUND_NUM).with_high_radius(HIGH_RADIUS);
	let leash = playground_leash(kind);
	scene.mob.intelligence.leash = leash;
	scene.mob.roster = MobRosterRecipe::from_kind(kind, PLAYGROUND_NUM, leash);
	scene
}

fn force_species(scene: &mut MobScene, species: CharacterSpecies) {
	for member in &mut scene.mob.roster.members {
		rewrite_member(member, species);
	}
	let Some(first) = scene.mob.roster.members.first().cloned() else {
		return;
	};
	while scene.mob.roster.members.len() < SPECIES_MEMBERS {
		scene.mob.roster.members.push(first.clone());
	}
	let count = scene.mob.roster.members.len();
	let radius = (scene.mob.intelligence.leash * 0.45).max(1.5);
	for (slot, member) in scene.mob.roster.members.iter_mut().enumerate() {
		let y = member.character.locomotion_capsule().spawn_height();
		let fraction = (slot as f32 + 0.5) / count.max(1) as f32;
		let r = radius * fraction.sqrt();
		let angle = slot as f32 * 2.399_963_1;
		member.offset = Vec3::new(angle.cos() * r, y, angle.sin() * r);
	}
}

fn rewrite_member(member: &mut maybraid_mobs::MobMemberRecipe, species: CharacterSpecies) {
	let recipe = member.character.as_ref();
	let character = MobCharacter {
		num: recipe.num,
		build: recipe.build,
		species,
		inventory: CharacterInventory::Empty,
		brains: recipe.brains,
	};
	let next = character.scene_recipe();
	member.offset.y = next.locomotion_capsule().spawn_height();
	member.character = Arc::new(next);
}

fn member_cap(placement: CastPlacement) -> usize {
	if placement.species.is_some() {
		SPECIES_MEMBERS
	} else {
		match placement.kind {
			MobKind::Pack => PACK_MEMBERS,
			_ => HERD_MEMBERS,
		}
	}
	.max(1)
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn herd_and_pack_rosters_stay_short() {
		let herd = scene_for(MobKind::Herd);
		assert!(!herd.mob.roster.members.is_empty());
		assert!(herd.mob.roster.members.len() <= HERD_MEMBERS);
		assert_eq!(herd.mob.kind, MobKind::Herd);

		let pack = scene_for(MobKind::Pack);
		assert!(!pack.mob.roster.members.is_empty());
		assert!(pack.mob.roster.members.len() <= PACK_MEMBERS);
		assert_eq!(pack.mob.kind, MobKind::Pack);
	}

	#[test]
	fn both_places_a_herd_and_a_pack() {
		let kinds: Vec<_> =
			PlaygroundCast::Both.placements().iter().map(|placement| placement.kind).collect();
		assert_eq!(kinds, vec![MobKind::Herd, MobKind::Pack]);
	}

	#[test]
	fn hars_and_ylter_casts_force_those_species() {
		let hars = scene_for_placement(PlaygroundCast::Hars.placements()[0]);
		assert_eq!(hars.mob.roster.members.len(), SPECIES_MEMBERS);
		assert!(hars
			.mob
			.roster
			.members
			.iter()
			.all(|member| member.character.species == CharacterSpecies::Hars));

		let ylter = scene_for_placement(PlaygroundCast::Ylter.placements()[0]);
		assert_eq!(ylter.mob.roster.members.len(), SPECIES_MEMBERS);
		assert!(ylter
			.mob
			.roster
			.members
			.iter()
			.all(|member| member.character.species == CharacterSpecies::Ylter));

		let species: Vec<_> = PlaygroundCast::HarsYlter
			.placements()
			.iter()
			.map(|placement| placement.species)
			.collect();
		assert_eq!(
			species,
			vec![Some(CharacterSpecies::Hars), Some(CharacterSpecies::Ylter)]
		);
	}

	#[test]
	fn playground_leashes_outgrow_personality_defaults() {
		assert!(scene_for(MobKind::Herd).mob.intelligence.leash > 24.0);
		assert!(scene_for(MobKind::Pack).mob.intelligence.leash > 16.0);
	}

	#[test]
	fn forage_ring_occupies_neighbor_journey_tiles() {
		let host = Vec2::new(-40.0, -20.0);
		let host_tile = (host / JOURNEY_TILE).floor().as_ivec2();
		let mut other = 0u32;
		for index in 0..8 {
			let angle = index as f32 / 8.0 * std::f32::consts::TAU;
			let at = Vec2::new(angle.cos() * 90.0, angle.sin() * 90.0);
			if (at / JOURNEY_TILE).floor().as_ivec2() != host_tile {
				other += 1;
			}
		}
		assert!(other >= 4, "host tile {host_tile:?} other={other}");
	}
}
