//! Character roster construction for each mob family.

use std::sync::Arc;

use bevy::prelude::*;
use mob_characters::{
	CharacterBrains, CharacterBuild, CharacterInventory, CharacterSceneRecipe, CharacterSpecies,
	FromMobNumber, MobCharacter,
};

use crate::MobKind;

#[derive(Clone, Debug, PartialEq)]
pub struct MobMemberRecipe {
	pub character: Arc<CharacterSceneRecipe>,
	pub offset: Vec3,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct MobRosterRecipe {
	pub members: Vec<MobMemberRecipe>,
}

impl MobRosterRecipe {
	pub fn from_kind(kind: MobKind, num: f32, leash: f32) -> Self {
		let seed = mix(u64::from(num.to_bits()) ^ kind as u64);
		let (min, max) = kind.count_range();
		let count = min + pick(seed, max - min + 1);
		let pack_species = pick_species(&CharacterSpecies::QUADRUPEDS, mix(seed ^ 0x0050_41CE));
		let ramble_bipeds = pick(seed ^ 0x02A6_B1E5, 2) == 0;
		let guard_common = if pick(seed ^ 0x6A4D, 4) < 3 {
			pick_species(&CharacterSpecies::BIPEDS, seed ^ 0xC011)
		} else {
			pick_species(&CharacterSpecies::QUADRUPEDS, seed ^ 0xC011)
		};
		let mut members = Vec::with_capacity(count);
		for slot in 0..count {
			let lane = mix(seed ^ (slot as u64 + 1).wrapping_mul(0x9E37_79B9));
			let member_num = num + slot as f32 * 0.754_877_7 + unit(lane);
			let species = species_for(kind, lane, pack_species, guard_common, ramble_bipeds);
			let brains = brains_for(kind, lane);
			let inventory = inventory_for(kind, species, lane);
			let build = build_for(member_num, lane);
			let character =
				MobCharacter { num: member_num, build, species, inventory, brains }.scene_recipe();
			let radius = (leash * 0.45).max(1.5);
			let offset =
				disk_offset(slot, count, radius, character.locomotion_capsule().spawn_height());
			members.push(MobMemberRecipe { character: Arc::new(character), offset });
		}
		Self { members }
	}
}

fn species_for(
	kind: MobKind,
	lane: u64,
	pack_species: CharacterSpecies,
	guard_common: CharacterSpecies,
	ramble_bipeds: bool,
) -> CharacterSpecies {
	if kind == MobKind::Pack {
		return pack_species;
	}
	if kind == MobKind::Guard {
		if pick(lane ^ 0xC0A0, 10) < 7 {
			return guard_common;
		}
		let values = if guard_common.is_biped() {
			&CharacterSpecies::BIPEDS[..]
		} else {
			&CharacterSpecies::QUADRUPEDS[..]
		};
		return pick_species(values, lane);
	}
	let bipeds = matches!(kind, MobKind::Raider | MobKind::Brawler)
		|| (kind == MobKind::Pleb && pick(lane ^ 0x91EB, 5) != 0)
		|| (kind == MobKind::Rambles && ramble_bipeds);
	if bipeds {
		pick_species(&CharacterSpecies::BIPEDS, lane)
	} else {
		pick_species(&CharacterSpecies::QUADRUPEDS, lane)
	}
}

fn brains_for(kind: MobKind, lane: u64) -> CharacterBrains {
	if matches!(kind, MobKind::Herd | MobKind::Guard) && pick(lane ^ 0xB2A1, 12) == 0 {
		return CharacterBrains::Roamer;
	}
	if kind == MobKind::Raider && pick(lane ^ 0xB2A1, 10) == 0 {
		return CharacterBrains::Roamer;
	}
	match kind {
		MobKind::Herd => CharacterBrains::Grazinger,
		MobKind::Pack => CharacterBrains::PackHunter,
		MobKind::Raider => CharacterBrains::Raider,
		MobKind::Guard => CharacterBrains::Guard,
		MobKind::Pleb => CharacterBrains::Civilian,
		MobKind::Rambles => CharacterBrains::Roamer,
		MobKind::Brawler => CharacterBrains::Brawler,
	}
}

fn inventory_for(kind: MobKind, species: CharacterSpecies, lane: u64) -> CharacterInventory {
	if species.is_quadruped() {
		return CharacterInventory::Empty;
	}
	let roll = pick(lane ^ 0x1A6B, 100);
	match kind {
		MobKind::Herd | MobKind::Pack => CharacterInventory::Empty,
		MobKind::Raider | MobKind::Guard => match roll {
			0..=59 => CharacterInventory::Grunt,
			60..=89 => CharacterInventory::Mercenary,
			_ => CharacterInventory::Specialist,
		},
		MobKind::Pleb => match roll {
			0..=9 => CharacterInventory::Empty,
			10..=89 => CharacterInventory::Clothed,
			_ => CharacterInventory::Flashy,
		},
		MobKind::Rambles => {
			if roll < 75 {
				CharacterInventory::Clothed
			} else {
				CharacterInventory::Grunt
			}
		}
		MobKind::Brawler => {
			if roll < 70 {
				CharacterInventory::Grunt
			} else {
				CharacterInventory::Mercenary
			}
		}
	}
}

fn build_for(num: f32, lane: u64) -> CharacterBuild {
	match pick(lane ^ 0xB017_D5E2, 100) {
		0..=44 => CharacterBuild::Base,
		45..=59 => CharacterBuild::from_num(num),
		60..=74 => CharacterBuild::Warrior,
		75..=84 => CharacterBuild::Renegade,
		85..=92 => CharacterBuild::Tank,
		93..=97 => CharacterBuild::Brawler,
		_ => CharacterBuild::Master,
	}
}

fn pick_species(values: &[CharacterSpecies], lane: u64) -> CharacterSpecies {
	values[pick(lane, values.len())]
}

fn disk_offset(slot: usize, count: usize, radius: f32, y: f32) -> Vec3 {
	let fraction = (slot as f32 + 0.5) / count.max(1) as f32;
	let r = radius * fraction.sqrt();
	let angle = slot as f32 * 2.399_963_1;
	Vec3::new(angle.cos() * r, y, angle.sin() * r)
}

fn pick(lane: u64, len: usize) -> usize {
	if len <= 1 {
		return 0;
	}
	(lane as usize) % len
}

fn unit(lane: u64) -> f32 {
	(lane >> 40) as f32 / (1_u32 << 24) as f32
}

fn mix(mut value: u64) -> u64 {
	value ^= value >> 30;
	value = value.wrapping_mul(0xBF58_476D_1CE4_E5B9);
	value ^= value >> 27;
	value = value.wrapping_mul(0x94D0_49BB_1331_11EB);
	value ^ (value >> 31)
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn family_counts_stay_in_the_sketch_ranges() {
		for kind in MobKind::VALUES {
			let roster = MobRosterRecipe::from_kind(kind, 17.25, 20.0);
			let (min, max) = kind.count_range();
			assert!((min..=max).contains(&roster.members.len()));
		}
	}

	#[test]
	fn pack_uses_one_quadruped_species() -> anyhow::Result<()> {
		let roster = MobRosterRecipe::from_kind(MobKind::Pack, 8.5, 18.0);
		let Some(first) = roster.members.first() else {
			anyhow::bail!("pack unexpectedly empty");
		};
		assert!(first.character.species.is_quadruped());
		assert!(roster.members.iter().all(|member| {
			member.character.species == first.character.species
				&& member.character.brains == CharacterBrains::PackHunter
		}));
		Ok(())
	}
}
