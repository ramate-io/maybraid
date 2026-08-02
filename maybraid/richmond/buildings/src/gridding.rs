use bevy_math::bounding::Aabb3d;

pub struct Box {
	pub bounds: Aabb3d,
	/// Horizontal roll which is applied to the primitives in the box after the fact.
	pub roll: f32,
}

/// A stacking object can determine the decision bounds on what can be stacked above the current storey.
pub trait Stacking<T> {
	/// Returns the bounding boxes of the boxes that are stacked above the current box.
	fn boxes_above(&self, storey: &T) -> Vec<Aabb3d>;
}
