use bevy::shape::Aabb3d;

#[derive(Debug, Clone, PartialEq)]
pub enum Bounds {
	Cuboid(Aabb3d),
	Unbounded,
}
