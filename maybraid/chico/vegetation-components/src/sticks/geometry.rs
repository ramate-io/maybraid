//! Stick continuous forms.

/// Stick footprint / role in the chain.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum StickGeometry {
	/// Generic branch / connector segment.
	#[default]
	Segment,
	/// Primary trunk segment (may prefer [`super::StickStyle::StandardTrunk`]).
	Trunk,
}

impl StickGeometry {
	pub fn segment() -> Self {
		Self::Segment
	}

	pub fn trunk() -> Self {
		Self::Trunk
	}
}
