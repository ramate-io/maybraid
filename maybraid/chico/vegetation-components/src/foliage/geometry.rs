//! Foliage continuous forms.

use crate::foliage::collection::FrondCollection;

/// Foliage footprint / construction.
///
/// Tessellated / multi-leaf forms (e.g. [`Self::FrondCollection`]) sit on the same
/// enum as single kits — one [`crate::FoliageNode`] and one foliage LOD probe, like
/// polyline partitions under a partition node.
#[derive(Debug, Clone, PartialEq)]
pub enum FoliageGeometry {
	/// Unit sphere centered at the origin (radius 1 before placement scale).
	UnitBall,
	/// Layered-ball kit: unit ball before placement scale (GLB under standard style).
	LayeredBall,
	/// Cheap-ball kit: lower-poly unit ball for dense packed clusters.
	CheapBall,
	/// Point tip kit (`straight_frond_001_*`); prefer [`Self::StraightFrondSegment`] for strands.
	StraightFrond,
	/// Square-ended frond segment (`straight_frond_segment_001_*`):
	/// \(Y \in [0, 1]\), \(X \in [-0.1, 0.1]\), \(Z\) negligible.
	StraightFrondSegment,
	/// Many placed frond kits under one LOD parent (merge thinning by distance).
	FrondCollection(FrondCollection),
	/// Plane-splay cluster parameters (local units before placement scale).
	PlaneSplay {
		icosphere_subdivisions: u32,
		core_radius: f32,
		leaf_disc_radius: f32,
	},
}

impl Default for FoliageGeometry {
	fn default() -> Self {
		Self::UnitBall
	}
}

impl FoliageGeometry {
	pub fn unit_ball() -> Self {
		Self::UnitBall
	}

	pub fn layered_ball() -> Self {
		Self::LayeredBall
	}

	pub fn cheap_ball() -> Self {
		Self::CheapBall
	}

	pub fn straight_frond() -> Self {
		Self::StraightFrond
	}

	pub fn straight_frond_segment() -> Self {
		Self::StraightFrondSegment
	}

	pub fn frond_collection(collection: FrondCollection) -> Self {
		Self::FrondCollection(collection)
	}

	pub fn plane_splay(
		icosphere_subdivisions: u32,
		core_radius: f32,
		leaf_disc_radius: f32,
	) -> Self {
		Self::PlaneSplay { icosphere_subdivisions, core_radius, leaf_disc_radius }
	}

	pub fn default_plane_splay() -> Self {
		Self::plane_splay(0, 0.8, 0.9)
	}

	pub fn is_frond_kit(&self) -> bool {
		matches!(self, Self::StraightFrond | Self::StraightFrondSegment)
	}

	pub fn is_frond_collection(&self) -> bool {
		matches!(self, Self::FrondCollection(_))
	}

	pub fn as_frond_collection(&self) -> Option<&FrondCollection> {
		match self {
			Self::FrondCollection(c) => Some(c),
			_ => None,
		}
	}
}
