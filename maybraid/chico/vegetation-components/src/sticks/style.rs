//! Stick look / kit backend.

/// Material / kit look for a stick segment.
///
/// SDF / procedural cylinders remain as named styles until GLBs replace them.
/// Trunk vs branch mesh choice is [`super::StickGeometry`], not a separate style.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum StickStyle {
	/// Procedural unit cylinder (stand-in for noisy SDF sticks).
	#[default]
	NoisyCylinder,
	/// GLB triads under `vegetation/sticks/standard/` (segment or trunk geometry).
	Standard,
}
