//! Foliage look / kit backend.

/// Material / kit look for a foliage cluster.
///
/// SDF / inline builders remain as named styles until GLBs replace them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FoliageStyle {
	/// Procedural unit sphere (stand-in for noisy SDF balls).
	NoisyBall,
	/// Inline icosphere + plate shell (plane splay).
	PlaneSplay,
}
