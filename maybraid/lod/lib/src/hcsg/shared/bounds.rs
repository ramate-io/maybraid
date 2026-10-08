//! [`HcsgBounds`]: where a generation or presentation system wants values.

use bevy::ecs::system::{SystemParam, SystemParamItem};
use bevy::math::bounding::Aabb3d;
use bevy::math::Vec3;

/// A bounds source: a camera, a gameplay region, an explicit warming region.
///
/// Every change to [`Self::inner`] replaces the system's subscription, so
/// snap it to the cells it covers rather than following the camera exactly.
/// Sources that should share hosts combine into one `HcsgBounds`.
pub trait HcsgBounds: Send + Sync + 'static {
	type Param: SystemParam + 'static;

	/// Region to fill and, for presentation, to spawn in. `None` requests
	/// nothing.
	fn inner(param: &SystemParamItem<Self::Param>) -> Option<Aabb3d>;

	/// Hosts are retired once their bounds leave this region, so it should
	/// contain [`Self::inner`] with some margin. `None` retires everything.
	fn outer(param: &SystemParamItem<Self::Param>) -> Option<Aabb3d>;

	/// Generation runs nearest this point first.
	fn focus(param: &SystemParamItem<Self::Param>) -> Option<Vec3> {
		let _ = param;
		None
	}
}
