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
	bounds: Aabb2d,
}

impl<T: LeftCollidable> LeftCollider<T> {
	pub fn new(collidable: T, bounds: Aabb2d) -> Self {
		Self { collidable, bounds }
	}

	pub fn collidable(&self) -> &T {
		&self.collidable
	}
}

/// Something can collide with this.
#[derive(Component)]
pub struct RightCollider<T: RightCollidable> {
	collidable: T,
	bounds: Aabb2d,
}

impl<T: RightCollidable> RightCollider<T> {
	pub fn new(collidable: T, bounds: Aabb2d) -> Self {
		Self { collidable, bounds }
	}

	pub fn collidable(&self) -> &T {
		&self.collidable
	}
}

pub struct CollisionPlugin<L: LeftCollidable, R: RightCollidable, C: CollisionLayer> {
	__marker: PhantomData<(L, R, C)>,
}

impl<L: LeftCollidable, R: RightCollidable, C: CollisionLayer> CollisionPlugin<L, R, C> {
	pub fn left_right_collisions(
		mut commands: Commands,
		left_query: Query<(Entity, &LeftCollider<L>, &C)>,
		right_query: Query<(Entity, &RightCollider<R>, &C)>,
	) {
		for (left_entity, left_collider, _layer) in left_query.iter() {
			for (right_entity, right_collider, _layer) in right_query.iter() {
				if left_entity == right_entity {
					continue;
				}

				if left_collider.bounds.intersects(&right_collider.bounds) {
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
