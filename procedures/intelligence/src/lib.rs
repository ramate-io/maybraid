use bevy::prelude::*;

pub mod local_pathfinding;

#[derive(Component)]
pub struct Move {
	pub to_position: Vec3,
}
