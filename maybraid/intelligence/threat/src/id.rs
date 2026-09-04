/// Stable identity for one potentially threatening subject.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ThreatId(pub u64);

/// Stable identity for a semantic affiliation group.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ThreatGroupId(pub u64);

impl ThreatGroupId {
	const INDIVIDUAL_BIT: u64 = 1 << 63;

	/// Caller-assigned shared group. The individual namespace is reserved.
	pub const fn group(id: u64) -> Self {
		Self(id & !Self::INDIVIDUAL_BIT)
	}

	/// Reserved singular group for one stable threat identity.
	pub const fn individual(id: ThreatId) -> Self {
		Self(id.0 | Self::INDIVIDUAL_BIT)
	}
}
