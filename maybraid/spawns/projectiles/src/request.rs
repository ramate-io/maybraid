//! Projectile spawn request. The payload is complete; the interpreter does
//! not rediscover the equipped item or current player.

use bevy::prelude::*;
use damage::HitPayload;
use maybraid_spawns_core::SpawnIdentity;
use projectiles::{BoltSpec, BulletSpec, ProjectileSource};

/// Ballistic load carried on the request. Lasers stay presentation-owned.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ProjectileSpec {
	Bolt(BoltSpec),
	Bullet(BulletSpec),
}

impl ProjectileSpec {
	pub fn ballistic(self) -> (f32, f32, f32, f32, f32, f32, Color, f32) {
		match self {
			Self::Bolt(spec) => (
				spec.length,
				spec.radius,
				spec.speed,
				spec.max_range,
				spec.penetration,
				spec.max_age,
				spec.color,
				0.0,
			),
			Self::Bullet(spec) => (
				spec.length,
				spec.radius,
				spec.speed,
				spec.max_range,
				spec.penetration,
				spec.max_age,
				spec.color,
				1.0,
			),
		}
	}
}

/// Complete launch. Origin and velocity are world-space.
#[derive(Message, Clone, Copy, Debug)]
pub struct SpawnProjectile {
	pub identity: SpawnIdentity,
	pub source: Entity,
	pub origin: Vec3,
	pub direction: Vec3,
	pub spec: ProjectileSpec,
	pub age: f32,
	pub payload: Option<HitPayload>,
}

impl SpawnProjectile {
	pub const fn new(
		identity: SpawnIdentity,
		source: Entity,
		origin: Vec3,
		direction: Vec3,
		spec: ProjectileSpec,
		age: f32,
		payload: Option<HitPayload>,
	) -> Self {
		Self { identity, source, origin, direction, spec, age, payload }
	}

	pub fn launch(
		source: Entity,
		origin: Vec3,
		direction: Vec3,
		spec: ProjectileSpec,
		age: f32,
		payload: Option<HitPayload>,
	) -> Self {
		Self::new(SpawnIdentity::default(), source, origin, direction, spec, age, payload)
	}

	pub fn projectile_source(self) -> ProjectileSource {
		ProjectileSource(self.source)
	}
}

/// Simulation entity created from a launch request.
#[derive(Message, Clone, Copy, Debug, PartialEq, Eq)]
pub struct ProjectileSpawned {
	pub identity: SpawnIdentity,
	pub projectile: Entity,
	pub source: Entity,
}

impl ProjectileSpawned {
	pub const fn new(identity: SpawnIdentity, projectile: Entity, source: Entity) -> Self {
		Self { identity, projectile, source }
	}
}
