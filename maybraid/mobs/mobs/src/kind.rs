use bevy::prelude::Component;
use mob_characters::FromMobNumber;

#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum MobKind {
	#[default]
	Herd,
	Pack,
	Raider,
	Guard,
	Pleb,
	Rambles,
	Brawler,
}

impl MobKind {
	pub const VALUES: [Self; 7] = [
		Self::Herd,
		Self::Pack,
		Self::Raider,
		Self::Guard,
		Self::Pleb,
		Self::Rambles,
		Self::Brawler,
	];

	pub const fn count_range(self) -> (usize, usize) {
		match self {
			Self::Herd => (1, 24),
			Self::Pack | Self::Raider | Self::Guard => (3, 12),
			Self::Pleb => (10, 24),
			Self::Rambles => (1, 12),
			Self::Brawler => (6, 12),
		}
	}
}

impl FromMobNumber for MobKind {
	fn from_num(num: f32) -> Self {
		let mixed = u64::from(num.to_bits()).wrapping_mul(0x9E37_79B9_7F4A_7C15);
		Self::VALUES[(mixed as usize) % Self::VALUES.len()]
	}
}
