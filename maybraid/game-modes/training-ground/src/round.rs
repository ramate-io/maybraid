//! Seeded Training lives. Schemes in later children read the map and site.

use bevy::prelude::*;
use durham_terrain_models::{fine_patch_cell_layout, TerrainCellLayout};

/// Half-extent of the training patch layout.
pub const TRAINING_FINE_HALF_EXTENT_CELLS: i32 = 2;
/// Sites land within this many 160 m cells of the world origin on each axis.
const TRAINING_SITE_RANGE_CELLS: i32 = 48;
/// Sites tried per round before Training gives up on stamping a development.
const TRAINING_SITE_ATTEMPTS: u32 = 8;
const TRAINING_SITE_SALT: u64 = 0x51_7E5A_17E5;
const TRAINING_DEVELOPMENT_SALT: u64 = 0xDE7E_10A5;
const TRAINING_MOB_SALT: u64 = 0x0B_5EED;
const TRAINING_TRAINEE_SALT: u64 = 0x7EA1_4EE5;
const TRAINING_NEXT_SALT: u64 = 0x9E37_79B9_7F4A_7C15;

/// One Training life. The shell rolls the first from entropy. A respawn
/// advances to [`Self::next`] (a fresh site, development, roster, and
/// trainee) or [`Self::next_life`] (the next trainee on the same map).
#[derive(Resource, Clone, Copy, Debug, PartialEq, Eq)]
pub struct TrainingRound {
	pub seed: u64,
	/// Sites already tried for this seed. A site that fits no development rerolls.
	site_attempt: u32,
	/// Lives played on this map.
	life: u32,
}

/// What a round stamps. Every life on the same map shares it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TrainingMap {
	seed: u64,
	site_attempt: u32,
}

impl Default for TrainingRound {
	fn default() -> Self {
		Self::new(42)
	}
}

impl TrainingRound {
	pub const fn new(seed: u64) -> Self {
		Self { seed, site_attempt: 0, life: 0 }
	}

	pub fn from_entropy() -> Self {
		let nanos = std::time::SystemTime::now()
			.duration_since(std::time::UNIX_EPOCH)
			.map(|elapsed| elapsed.as_nanos() as u64)
			.unwrap_or(0);
		Self::new(mix(nanos))
	}

	/// A new map after this life.
	pub fn next(self) -> Self {
		Self::new(self.lane(TRAINING_NEXT_SALT))
	}

	/// The next life on this map.
	pub fn next_life(self) -> Self {
		Self { life: self.life.wrapping_add(1), ..self }
	}

	pub fn map(self) -> TrainingMap {
		TrainingMap { seed: self.seed, site_attempt: self.site_attempt }
	}

	pub fn life(self) -> u32 {
		self.life
	}

	pub fn reroll_site(self) -> Self {
		Self { site_attempt: self.site_attempt + 1, ..self }
	}

	pub fn site_exhausted(self) -> bool {
		self.site_attempt + 1 >= TRAINING_SITE_ATTEMPTS
	}

	/// Cell corner the patch centers on.
	pub fn site(self) -> IVec2 {
		let lane = self.lane(TRAINING_SITE_SALT ^ u64::from(self.site_attempt).rotate_left(17));
		let span = (2 * TRAINING_SITE_RANGE_CELLS + 1) as u64;
		let axis = |bits: u64| (bits % span) as i32 - TRAINING_SITE_RANGE_CELLS;
		IVec2::new(axis(lane), axis(lane >> 32))
	}

	pub fn layout(self) -> TerrainCellLayout {
		let half = TRAINING_FINE_HALF_EXTENT_CELLS;
		fine_patch_cell_layout(half, self.site() - IVec2::splat(half))
	}

	pub fn development_seed(self) -> u32 {
		self.lane(TRAINING_DEVELOPMENT_SALT) as u32
	}

	/// Mob number for the first squad. 24 bits, so every squad offset stays exact.
	pub fn mob_seed(self) -> f32 {
		(self.lane(TRAINING_MOB_SALT) >> 40) as f32
	}

	/// Entropy for the world's trainee roll. The loadout type lives in the world.
	pub fn trainee_seed(self) -> u64 {
		let life = u64::from(self.life).rotate_left(23);
		self.lane(TRAINING_TRAINEE_SALT ^ life)
	}

	fn lane(self, salt: u64) -> u64 {
		mix(self.seed ^ salt)
	}
}

fn mix(value: u64) -> u64 {
	let mut value = value.wrapping_add(0x9E37_79B9_7F4A_7C15);
	value = (value ^ (value >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
	value = (value ^ (value >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
	value ^ (value >> 31)
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn rounds_are_reproducible_and_move_on() {
		let round = TrainingRound::new(1234);
		assert_eq!(round.next(), TrainingRound::new(1234).next());
		assert_ne!(round.next().seed, round.seed);
		let sites: std::collections::HashSet<IVec2> =
			std::iter::successors(Some(round), |round| Some(round.next()))
				.take(16)
				.map(TrainingRound::site)
				.collect();
		assert!(sites.len() > 8, "sixteen lives should not share a handful of sites");
		for site in sites {
			assert!(site.abs().max_element() <= TRAINING_SITE_RANGE_CELLS);
		}
	}

	#[test]
	fn site_rerolls_keep_the_seed_and_stop() {
		let mut round = TrainingRound::new(99);
		let first = round.site();
		round = round.reroll_site();
		assert_eq!(round.seed, 99);
		assert_ne!(round.site(), first);
		let tries = std::iter::successors(Some(TrainingRound::new(99)), |round| {
			(!round.site_exhausted()).then(|| round.reroll_site())
		})
		.count();
		assert_eq!(tries as u32, TRAINING_SITE_ATTEMPTS);
	}
}
