use bevy::math::bounding::{Aabb2d, IntersectsVolume};
use bevy::prelude::*;
use std::marker::PhantomData;

pub trait CollisionLayer: Component {}

pub trait LeftCollidable: Component {
	/// Spawns the left-sided response to a collision with the right entity.
	fn spawn_left_collision_entity(&self, commands: &mut Commands, right: Entity) -> Entity;
}

pub trait RightCollidable: Component {
	/// Spawns the right-sided response to a collision with the left entity.
	fn spawn_right_collision_entity(&self, commands: &mut Commands, left: Entity) -> Entity;
}

/// This can collide with something.
#[derive(Component)]
pub struct LeftCollider<T: LeftCollidable> {
	collidable: T,
	size_max: Vec2,
}

impl<T: LeftCollidable> LeftCollider<T> {
	pub fn new(collidable: T, size_max: Vec2) -> Self {
		Self { collidable, size_max }
	}

	pub fn collidable(&self) -> &T {
		&self.collidable
	}
}

/// Something can collide with this.
#[derive(Component)]
pub struct RightCollider<T: RightCollidable> {
	collidable: T,
	size_max: Vec2,
}

impl<T: RightCollidable> RightCollider<T> {
	pub fn new(collidable: T, size_max: Vec2) -> Self {
		Self { collidable, size_max }
	}

	pub fn collidable(&self) -> &T {
		&self.collidable
	}
}

pub struct CollisionPlugin<L: LeftCollidable, R: RightCollidable, C: CollisionLayer> {
	__marker: PhantomData<(L, R, C)>,
}

impl<L: LeftCollidable, R: RightCollidable, C: CollisionLayer> Default
	for CollisionPlugin<L, R, C>
{
	fn default() -> Self {
		Self { __marker: PhantomData }
	}
}

impl<L: LeftCollidable, R: RightCollidable, C: CollisionLayer> CollisionPlugin<L, R, C> {
	pub fn left_right_collisions(
		mut commands: Commands,
		left_query: Query<(Entity, &LeftCollider<L>, &C, &Transform)>,
		right_query: Query<(Entity, &RightCollider<R>, &C, &Transform)>,
	) {
		for (left_entity, left_collider, _layer, left_transform) in left_query.iter() {
			for (right_entity, right_collider, _layer, right_transform) in right_query.iter() {
				if left_entity == right_entity {
					continue;
				}

				let left_bounds = Aabb2d::new(
					left_transform.translation.xy() + left_collider.size_max / 2.0,
					left_collider.size_max / 2.0,
				);

				let right_bounds = Aabb2d::new(
					right_transform.translation.xy() + right_collider.size_max / 2.0,
					right_collider.size_max / 2.0,
				);

				if left_bounds.intersects(&right_bounds) {
					left_collider
						.collidable()
						.spawn_left_collision_entity(&mut commands, right_entity);
					right_collider
						.collidable()
						.spawn_right_collision_entity(&mut commands, left_entity);
				}
			}
		}
	}
}

impl<L: LeftCollidable, R: RightCollidable, C: CollisionLayer> Plugin for CollisionPlugin<L, R, C> {
	fn build(&self, app: &mut App) {
		app.add_systems(Update, Self::left_right_collisions);
	}
}
