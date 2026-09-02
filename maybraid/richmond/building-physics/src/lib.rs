//! Fixed-layer walk colliders from Richmond domain IR.
//!
//! LOD hosts stay on [`lod_avian::PhysicsInteractionLayer::Host`]. Walk geometry is
//! spawned as child [`RigidBody::Static`] cuboids on
//! [`lod_avian::PhysicsInteractionLayer::Fixed`] so movers can stand on floors and
//! stair ramps without treating Host volumes as contact.

mod colliders;

use avian3d::prelude::{CoefficientCombine, Friction};
use bevy::prelude::*;

pub use colliders::{spawn_building_walk_colliders, BuildingWalkCollider};

/// Dirt / stone grip. [`CoefficientCombine::Max`] beats the character controller's
/// `Friction::ZERO` + `Min`.
pub const BUILDING_FRICTION: Friction = Friction {
	dynamic_coefficient: 0.75,
	static_coefficient: 0.95,
	combine_rule: CoefficientCombine::Max,
};

/// Friction applied to new building walk cuboids.
#[derive(Resource, Clone, Copy, Debug, PartialEq)]
pub struct BuildingFrictionConfig(pub Friction);

impl Default for BuildingFrictionConfig {
	fn default() -> Self {
		Self(BUILDING_FRICTION)
	}
}

/// Registers [`BuildingFrictionConfig`]. Colliders are stamped at spawn via
/// [`spawn_building_walk_colliders`].
pub struct BuildingWalkColliderPlugin;

impl Plugin for BuildingWalkColliderPlugin {
	fn build(&self, app: &mut App) {
		app.init_resource::<BuildingFrictionConfig>();
	}
}
