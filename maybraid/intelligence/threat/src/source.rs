use std::ops::{BitOr, BitOrAssign};

/// Independent reasons supporting one retained threat.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct ThreatSource(u8);

impl ThreatSource {
	pub const LOCAL_SCAN: Self = Self(1 << 0);
	pub const SESSION: Self = Self(1 << 1);
	pub const RECEIVED_FIRE: Self = Self(1 << 2);
	pub const RECEIVED_DAMAGE: Self = Self(1 << 3);
	pub const SHARED: Self = Self(1 << 4);
	pub const OBJECTIVE: Self = Self(1 << 5);

	/// Bits that may be written up to a pack board. `SHARED` is derived only.
	pub const FIRST_HAND: Self = Self(
		Self::LOCAL_SCAN.0
			| Self::SESSION.0
			| Self::RECEIVED_FIRE.0
			| Self::RECEIVED_DAMAGE.0
			| Self::OBJECTIVE.0,
	);

	pub const fn is_empty(self) -> bool {
		self.0 == 0
	}

	pub const fn contains(self, other: Self) -> bool {
		(self.0 & other.0) == other.0
	}

	pub const fn intersects(self, other: Self) -> bool {
		(self.0 & other.0) != 0
	}

	pub const fn is_first_hand(self) -> bool {
		self.intersects(Self::FIRST_HAND)
	}

	pub fn insert(&mut self, other: Self) {
		self.0 |= other.0;
	}

	pub fn remove(&mut self, other: Self) {
		self.0 &= !other.0;
	}
}

impl BitOr for ThreatSource {
	type Output = Self;

	fn bitor(self, rhs: Self) -> Self::Output {
		Self(self.0 | rhs.0)
	}
}

impl BitOrAssign for ThreatSource {
	fn bitor_assign(&mut self, rhs: Self) {
		self.insert(rhs);
	}
}

#[cfg(test)]
mod tests {
	use super::ThreatSource;

	#[test]
	fn first_hand_excludes_shared_only() -> anyhow::Result<()> {
		assert!(ThreatSource::LOCAL_SCAN.is_first_hand());
		assert!(ThreatSource::RECEIVED_DAMAGE.is_first_hand());
		assert!(ThreatSource::OBJECTIVE.is_first_hand());
		assert!(!(ThreatSource::SHARED.is_first_hand()));
		assert!((ThreatSource::LOCAL_SCAN | ThreatSource::SHARED).is_first_hand());
		Ok(())
	}
}
