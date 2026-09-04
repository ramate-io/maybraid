use std::ops::{BitAnd, BitAndAssign, BitOr, BitOrAssign, Not};

/// Semantic reasons that an entity belongs to the assailant set.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct AssailantSource(u8);

impl AssailantSource {
	pub const NONE: Self = Self(0);
	pub const SPOTTING: Self = Self(1 << 0);
	pub const RECEIVED_FIRE: Self = Self(1 << 1);
	pub const ENEMYSHIP: Self = Self(1 << 2);
	pub const ALL: Self = Self(Self::SPOTTING.0 | Self::RECEIVED_FIRE.0 | Self::ENEMYSHIP.0);

	pub const fn from_bits(bits: u8) -> Self {
		Self(bits & Self::ALL.0)
	}

	pub const fn bits(self) -> u8 {
		self.0
	}

	pub const fn is_empty(self) -> bool {
		self.0 == 0
	}

	pub const fn contains(self, other: Self) -> bool {
		self.0 & other.0 == other.0
	}

	pub fn insert(&mut self, other: Self) {
		self.0 |= other.0;
	}

	pub fn remove(&mut self, other: Self) {
		self.0 &= !other.0;
	}
}

impl BitOr for AssailantSource {
	type Output = Self;

	fn bitor(self, rhs: Self) -> Self::Output {
		Self(self.0 | rhs.0)
	}
}

impl BitOrAssign for AssailantSource {
	fn bitor_assign(&mut self, rhs: Self) {
		self.insert(rhs);
	}
}

impl BitAnd for AssailantSource {
	type Output = Self;

	fn bitand(self, rhs: Self) -> Self::Output {
		Self(self.0 & rhs.0)
	}
}

impl BitAndAssign for AssailantSource {
	fn bitand_assign(&mut self, rhs: Self) {
		self.0 &= rhs.0;
	}
}

impl Not for AssailantSource {
	type Output = Self;

	fn not(self) -> Self::Output {
		Self(!self.0 & Self::ALL.0)
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn source_masks_compose() -> anyhow::Result<()> {
		let mut sources = AssailantSource::ENEMYSHIP | AssailantSource::SPOTTING;
		assert!(sources.contains(AssailantSource::ENEMYSHIP));
		sources.remove(AssailantSource::SPOTTING);
		assert_eq!(sources, AssailantSource::ENEMYSHIP);
		Ok(())
	}
}
