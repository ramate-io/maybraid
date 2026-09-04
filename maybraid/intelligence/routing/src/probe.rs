use bevy::prelude::*;

/// World queries used while scoring a chord. Implementations snap at probe time;
/// the plan stores the resulting snapshots.
pub trait RouteProbe {
	/// Surface point under `xz`, or `None` if there is no walkable ground.
	fn ground(&self, xz: Vec2, hint_y: f32) -> Option<Vec3>;

	/// Whether a hip-height chord hits Fixed geometry before the far end.
	fn blocked(&self, from_hip: Vec3, to_hip: Vec3) -> bool;
}
