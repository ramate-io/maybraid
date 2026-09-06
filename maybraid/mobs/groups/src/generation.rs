//! Environment-weighted group selection over 400 m generation bounds.

use bevy::prelude::*;
use maybraid_mobs::{MobKind, MobScene};
use mob_characters::FromMobNumber;

pub const DEFAULT_GROUP_EXTENT: f32 = 400.0;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct MobEnvironmentSample {
	pub elevation: Option<f32>,
	pub urbanization: f32,
	pub vegetation: f32,
}

/// Runtime adapter seam for Richmond development and Chico vegetation models.
pub trait MobWorldSample {
	fn sample_mobs(&self, xz: Vec2) -> MobEnvironmentSample;
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum GroupKind {
	#[default]
	Peaceful,
	Wild,
	Frontier,
	Warfront,
	Dystopian,
}

impl GroupKind {
	pub const VALUES: [Self; 5] =
		[Self::Peaceful, Self::Wild, Self::Frontier, Self::Warfront, Self::Dystopian];

	pub const fn count_range(self) -> (usize, usize) {
		match self {
			Self::Wild => (2, 8),
			Self::Peaceful | Self::Frontier | Self::Warfront | Self::Dystopian => (2, 12),
		}
	}
}

impl FromMobNumber for GroupKind {
	fn from_num(num: f32) -> Self {
		let bits = num.to_bits().wrapping_mul(0x9E37_79B9);
		Self::VALUES[bits as usize % Self::VALUES.len()]
	}
}

#[derive(Clone, Debug)]
pub struct PlacedMob {
	pub scene: MobScene,
	pub transform: Transform,
	pub environment: MobEnvironmentSample,
}

#[derive(Clone, Debug, Default)]
pub struct MobGroup {
	pub kind: GroupKind,
	pub seed: u64,
	pub origin: Vec2,
	pub extent: f32,
	pub mobs: Vec<PlacedMob>,
}

impl MobGroup {
	pub fn generate(kind: GroupKind, seed: u64, origin: Vec2, world: &impl MobWorldSample) -> Self {
		let mut rng = GroupRng::new(seed);
		let (min, max) = kind.count_range();
		let wanted = rng.in_range(min, max);
		let mut mobs = Vec::with_capacity(wanted);
		let mut probes = 0;
		while mobs.len() < wanted && probes < wanted.saturating_mul(16) {
			probes += 1;
			let xz = origin
				+ Vec2::new(
					(rng.unit() - 0.5) * DEFAULT_GROUP_EXTENT,
					(rng.unit() - 0.5) * DEFAULT_GROUP_EXTENT,
				);
			let environment = world.sample_mobs(xz);
			let Some(elevation) = environment.elevation else {
				continue;
			};
			let mob_kind = choose_mob(kind, environment, &mut rng);
			let num = rng.unit() * 1_000_000.0 + mobs.len() as f32;
			mobs.push(PlacedMob {
				scene: MobScene::of_kind(mob_kind, num),
				transform: Transform::from_xyz(xz.x, elevation, xz.y),
				environment,
			});
		}
		Self { kind, seed, origin, extent: DEFAULT_GROUP_EXTENT, mobs }
	}

	pub fn spawn(&self, commands: &mut Commands) -> Vec<Entity> {
		self.mobs
			.iter()
			.map(|placed| placed.scene.spawn(commands, placed.transform))
			.collect()
	}
}

fn choose_mob(group: GroupKind, sample: MobEnvironmentSample, rng: &mut GroupRng) -> MobKind {
	let urban = sample.urbanization.clamp(0.0, 1.0);
	let vegetation = sample.vegetation.clamp(0.0, 1.0);
	let weights: &[(MobKind, f32)] = match group {
		GroupKind::Peaceful => &[
			(MobKind::Pleb, 0.2 + urban * 4.0),
			(MobKind::Herd, 0.2 + vegetation * 4.0),
			(MobKind::Rambles, 0.35),
		],
		GroupKind::Wild => &[
			(MobKind::Raider, 0.7 + urban),
			(MobKind::Herd, 0.7 + vegetation),
			(MobKind::Pack, 0.65 + vegetation),
			(MobKind::Rambles, 0.7),
		],
		GroupKind::Frontier => &[
			(MobKind::Guard, 0.2 + urban * 2.2),
			(MobKind::Raider, 0.2 + urban * 1.8),
			(MobKind::Brawler, 0.15 + urban * 1.5),
			(MobKind::Herd, 0.2 + vegetation * 2.0),
			(MobKind::Pack, 0.15 + vegetation * 1.4),
		],
		GroupKind::Warfront => &[
			(MobKind::Guard, 0.2 + urban * 2.0),
			(MobKind::Raider, 0.2 + urban * 2.0),
			(MobKind::Pleb, 0.2 + urban),
			(MobKind::Brawler, 0.2 + urban * 1.5),
			(MobKind::Herd, 0.15 + vegetation),
			(MobKind::Pack, 0.2 + vegetation * 1.5),
		],
		GroupKind::Dystopian => &[
			(MobKind::Guard, 0.25 + urban * 3.0),
			(MobKind::Pleb, 0.25 + urban * 2.2),
			(MobKind::Herd, 0.15 + vegetation * 2.0),
		],
	};
	let total: f32 = weights.iter().map(|(_, weight)| *weight).sum();
	let mut throw = rng.unit() * total;
	for (kind, weight) in weights {
		if throw <= *weight {
			return *kind;
		}
		throw -= *weight;
	}
	weights.last().map(|(kind, _)| *kind).unwrap_or(MobKind::Rambles)
}

#[derive(Clone, Copy, Debug)]
struct GroupRng(u64);

impl GroupRng {
	fn new(seed: u64) -> Self {
		Self(seed.max(1))
	}

	fn next(&mut self) -> u64 {
		let mut value = self.0;
		value ^= value >> 12;
		value ^= value << 25;
		value ^= value >> 27;
		self.0 = value;
		value.wrapping_mul(0x2545_F491_4F6C_DD1D)
	}

	fn unit(&mut self) -> f32 {
		(self.next() >> 40) as f32 / (1_u32 << 24) as f32
	}

	fn in_range(&mut self, min: usize, max: usize) -> usize {
		if max <= min {
			return min;
		}
		min + self.next() as usize % (max - min + 1)
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	struct FlatWorld(MobEnvironmentSample);

	impl MobWorldSample for FlatWorld {
		fn sample_mobs(&self, _xz: Vec2) -> MobEnvironmentSample {
			self.0
		}
	}

	#[test]
	fn generation_respects_group_count_ranges() {
		let world = FlatWorld(MobEnvironmentSample {
			elevation: Some(4.0),
			urbanization: 0.5,
			vegetation: 0.5,
		});
		for kind in GroupKind::VALUES {
			let group = MobGroup::generate(kind, 17, Vec2::ZERO, &world);
			let (min, max) = kind.count_range();
			assert!((min..=max).contains(&group.mobs.len()));
		}
	}

	#[test]
	fn missing_surface_rejects_spawn_points() {
		let world = FlatWorld(MobEnvironmentSample::default());
		let group = MobGroup::generate(GroupKind::Peaceful, 2, Vec2::ZERO, &world);
		assert!(group.mobs.is_empty());
	}

	#[test]
	fn peaceful_group_never_selects_hostile_mobs() {
		let world = FlatWorld(MobEnvironmentSample {
			elevation: Some(0.0),
			urbanization: 1.0,
			vegetation: 1.0,
		});
		let group = MobGroup::generate(GroupKind::Peaceful, 99, Vec2::ZERO, &world);
		assert!(group.mobs.iter().all(|mob| matches!(
			mob.scene.mob.kind,
			MobKind::Pleb | MobKind::Herd | MobKind::Rambles
		)));
	}
}
