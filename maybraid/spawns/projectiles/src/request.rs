//! Compact projectile spawn request. The payload is complete; the interpreter
//! does not rediscover the equipped item or current player.
//!
//! No request identity and no per-shot bookkeeping entity. Outcomes are not
//! emitted unless a consumer asks for the created entity.

use bevy::prelude::*;
use damage::HitPayload;
use projectiles::ProjectileSource;

/// Ballistic load carried on the request. Lasers stay presentation-owned.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ProjectileSpec {
	Bolt(projectiles::BoltSpec),
	Bullet(projectiles::BulletSpec),
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
	pub source: Entity,
	pub origin: Vec3,
	pub direction: Vec3,
	pub spec: ProjectileSpec,
	pub age: f32,
	pub payload: Option<HitPayload>,
}

impl SpawnProjectile {
	pub const fn launch(
		source: Entity,
		origin: Vec3,
		direction: Vec3,
		spec: ProjectileSpec,
		age: f32,
		payload: Option<HitPayload>,
	) -> Self {
		Self { source, origin, direction, spec, age, payload }
	}

	pub fn projectile_source(self) -> ProjectileSource {
		ProjectileSource(self.source)
	}
}
