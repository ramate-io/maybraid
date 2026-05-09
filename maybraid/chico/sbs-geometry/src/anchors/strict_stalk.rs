//! **Strict stalk** segments in the **shared** [`crate::BallStickChain`]: hysteresis that keeps growth collinear (no lateral branch wander) so the trunk reads as a straight run from ground toward the anchor band.
//!
//! Used when [`super::Anchors`] emits the vertical chain of seed nodes along the stalk **radial centroid**; canopy ring seeds on that same axis then branch under the normal [`crate::ChainHysteresisRule`].

use bevy_math::Vec3;

#[derive(Clone, Debug, PartialEq)]
pub struct StrictStalk {
	pub height: f32,
	pub base_anchor: Vec3,
	pub base_radius: f32,
}

impl StrictStalk {
	/// Stalk radial centroid at height `t * height` above [`Self::base_anchor`], with `t` in `[0, 1]`.
	pub fn centroid_at_height_fraction(&self, t: f32) -> Vec3 {
		let t = t.clamp(0.0, 1.0);
		self.base_anchor + Vec3::Y * (t * self.height)
	}
}
