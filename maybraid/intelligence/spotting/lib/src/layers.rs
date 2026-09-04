use std::ops::{BitOr, BitOrAssign};

/// Semantic categories used to match spotting directives to subjects.
///
/// These bits are independent of physics collision layers.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct InterestLayers(u32);

impl InterestLayers {
	pub const CHARACTER: Self = Self(1 << 0);
	pub const WEAPON: Self = Self(1 << 1);
	pub const VEGETATION: Self = Self(1 << 2);
	pub const COMMERCE: Self = Self(1 << 3);
	pub const LANDMARK: Self = Self(1 << 4);

	pub const NONE: Self = Self(0);
	pub const ALL: Self = Self(
		Self::CHARACTER.0
			| Self::WEAPON.0
			| Self::VEGETATION.0
			| Self::COMMERCE.0
			| Self::LANDMARK.0,
	);

	pub const fn from_bits(bits: u32) -> Self {
		Self(bits)
	}

	pub const fn bits(self) -> u32 {
		self.0
	}

	pub const fn is_empty(self) -> bool {
		self.0 == 0
	}

	pub const fn intersects(self, other: Self) -> bool {
		self.0 & other.0 != 0
	}
}

impl BitOr for InterestLayers {
	type Output = Self;

	fn bitor(self, rhs: Self) -> Self::Output {
		Self(self.0 | rhs.0)
	}
}

impl BitOrAssign for InterestLayers {
	fn bitor_assign(&mut self, rhs: Self) {
		self.0 |= rhs.0;
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn semantic_layers_compose_and_intersect() -> anyhow::Result<()> {
		let things = InterestLayers::CHARACTER | InterestLayers::WEAPON;
		assert!(things.intersects(InterestLayers::CHARACTER));
		assert!(things.intersects(InterestLayers::WEAPON));
		assert!(!things.intersects(InterestLayers::VEGETATION));
		assert_eq!(things.bits(), InterestLayers::CHARACTER.bits() | InterestLayers::WEAPON.bits());
		Ok(())
	}
}
