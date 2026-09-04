use std::ops::{BitAnd, BitAndAssign, BitOr, BitOrAssign, Not};

/// Independent discovery mechanisms that currently support a known POI.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct PoiSource(u8);

impl PoiSource {
	pub const LOCAL_SCAN: Self = Self(1 << 0);
	pub const GLOBAL_SCAN: Self = Self(1 << 1);
	pub const EXTERNAL: Self = Self(1 << 2);
	pub const SHARED: Self = Self(1 << 3);
	pub const OBJECTIVE: Self = Self(1 << 4);

	pub const fn empty() -> Self {
		Self(0)
	}

	pub const fn is_empty(self) -> bool {
		self.0 == 0
	}

	pub const fn contains(self, other: Self) -> bool {
		(self.0 & other.0) == other.0
	}

	pub const fn intersects(self, other: Self) -> bool {
		(self.0 & other.0) != 0
	}

	pub fn insert(&mut self, other: Self) {
		self.0 |= other.0;
	}

	pub fn remove(&mut self, other: Self) {
		self.0 &= !other.0;
	}
}

impl BitOr for PoiSource {
	type Output = Self;

	fn bitor(self, rhs: Self) -> Self::Output {
		Self(self.0 | rhs.0)
	}
}

impl BitOrAssign for PoiSource {
	fn bitor_assign(&mut self, rhs: Self) {
		self.0 |= rhs.0;
	}
}

impl BitAnd for PoiSource {
	type Output = Self;

	fn bitand(self, rhs: Self) -> Self::Output {
		Self(self.0 & rhs.0)
	}
}

impl BitAndAssign for PoiSource {
	fn bitand_assign(&mut self, rhs: Self) {
		self.0 &= rhs.0;
	}
}

impl Not for PoiSource {
	type Output = Self;

	fn not(self) -> Self::Output {
		Self(!self.0)
	}
}
