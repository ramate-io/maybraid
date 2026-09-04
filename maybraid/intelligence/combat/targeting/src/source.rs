use std::ops::{BitAnd, BitAndAssign, BitOr, BitOrAssign, BitXor, BitXorAssign, Not};

/// Semantic reasons that an entity belongs to the active target set.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct TargetSource(u8);

impl TargetSource {
	pub const NONE: Self = Self(0);
	pub const OBJECTIVE: Self = Self(1 << 0);
	pub const SPOTTING: Self = Self(1 << 1);
	pub const RECEIVED_FIRE: Self = Self(1 << 2);
	pub const ALLY: Self = Self(1 << 3);
	pub const ENEMYSHIP: Self = Self(1 << 4);
	pub const FIREARM: Self = Self(1 << 5);
	pub const ALL: Self = Self(
		Self::OBJECTIVE.0
			| Self::SPOTTING.0
			| Self::RECEIVED_FIRE.0
			| Self::ALLY.0
			| Self::ENEMYSHIP.0
			| Self::FIREARM.0,
	);

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

	pub const fn intersects(self, other: Self) -> bool {
		self.0 & other.0 != 0
	}

	pub fn insert(&mut self, other: Self) {
		self.0 |= other.0;
	}

	pub fn remove(&mut self, other: Self) {
		self.0 &= !other.0;
	}
}

impl BitOr for TargetSource {
	type Output = Self;

	fn bitor(self, rhs: Self) -> Self::Output {
		Self(self.0 | rhs.0)
	}
}

impl BitOrAssign for TargetSource {
	fn bitor_assign(&mut self, rhs: Self) {
		self.insert(rhs);
	}
}

impl BitAnd for TargetSource {
	type Output = Self;

	fn bitand(self, rhs: Self) -> Self::Output {
		Self(self.0 & rhs.0)
	}
}

impl BitAndAssign for TargetSource {
	fn bitand_assign(&mut self, rhs: Self) {
		self.0 &= rhs.0;
	}
}

impl BitXor for TargetSource {
	type Output = Self;

	fn bitxor(self, rhs: Self) -> Self::Output {
		Self(self.0 ^ rhs.0)
	}
}

impl BitXorAssign for TargetSource {
	fn bitxor_assign(&mut self, rhs: Self) {
		self.0 ^= rhs.0;
	}
}

impl Not for TargetSource {
	type Output = Self;

	fn not(self) -> Self::Output {
		Self(!self.0 & Self::ALL.0)
	}
}

#[cfg(test)]
mod tests {
	use crate::TargetSource;

	#[test]
	fn source_masks_support_semantic_membership() -> anyhow::Result<()> {
		let mut sources = TargetSource::OBJECTIVE | TargetSource::SPOTTING;
		assert!(sources.contains(TargetSource::OBJECTIVE));
		assert!(sources.intersects(TargetSource::SPOTTING | TargetSource::FIREARM));

		sources.remove(TargetSource::SPOTTING);
		assert_eq!(sources, TargetSource::OBJECTIVE);
		assert_eq!((!TargetSource::OBJECTIVE).bits(), TargetSource::ALL.bits() ^ 1);
		Ok(())
	}
}
