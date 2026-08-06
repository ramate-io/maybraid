//! Foliage continuous forms.

/// Foliage footprint / construction.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FoliageGeometry {
	/// Unit sphere centered at the origin (radius 1 before placement scale).
	UnitBall,
	/// Layered-ball kit: unit ball before placement scale (GLB under standard style).
	LayeredBall,
	/// Cheap-ball kit: lower-poly unit ball for dense packed clusters.
	CheapBall,
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
}
