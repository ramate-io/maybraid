//! Synthesized targets other systems query. Item users write the aim/use slots.

use bevy::ecs::query::Has;
use bevy::prelude::*;
use crozon_characters::{ApplyTerrainPitch, TerrainPitchUsesVisualYaw};

#[derive(Component)]
pub struct Player;

/// Capsule agent that is not the followed player. Shares look / pose slots, not pad or camera.
#[derive(Component)]
pub struct Npc;

#[derive(Component)]
pub struct CameraFollow;

/// Nested character host on the player capsule.
#[derive(Component)]
pub struct PlayerVisual;

#[derive(Component)]
pub struct PlayerCapsule;

/// Look copied off the follow camera. Item users read this instead of `Camera3d`.
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct PlayerLook {
	pub yaw: f32,
	pub pitch: f32,
	pub first_person: bool,
	pub focus: f32,
}

/// Camera override written by a use-driver (sights, lock-on). Follow lerps by `focus`.
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct PlayerCameraAim {
	pub pose: Option<PlayerCameraPose>,
	pub focus: f32,
}

#[derive(Clone, Copy, Debug)]
pub struct PlayerCameraPose {
	pub translation: Vec3,
	pub rotation: Quat,
}

impl PlayerCameraPose {
	pub fn interpolate(self, other: Self, amount: f32) -> Self {
		Self {
			translation: self.translation.lerp(other.translation, amount),
			rotation: self.rotation.slerp(other.rotation, amount),
		}
	}

	pub fn transform(self) -> Transform {
		Transform::from_translation(self.translation).with_rotation(self.rotation)
	}
}

/// Who owns [`PlayerVisual`] yaw: locomotion wish, or first-person look cone.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PlayerYawOwner {
	#[default]
	Wish,
	Look,
}

/// Look-owned pitched visuals skip stored `yaw_facing` so mouse look is not gated.
pub(crate) fn sync_terrain_pitch_visual_yaw(
	mut commands: Commands,
	owners: Query<
		(Entity, &PlayerYawOwner, Has<TerrainPitchUsesVisualYaw>),
		With<ApplyTerrainPitch>,
	>,
) {
	for (entity, owner, uses_visual) in &owners {
		match *owner {
			PlayerYawOwner::Look if !uses_visual => {
				commands.entity(entity).insert(TerrainPitchUsesVisualYaw);
			}
			PlayerYawOwner::Wish if uses_visual => {
				commands.entity(entity).remove::<TerrainPitchUsesVisualYaw>();
			}
			_ => {}
		}
	}
}

/// Active extra driver (held gun, melee, …). Camera/pose overlays key off this.
#[derive(Component, Clone, Copy, Debug)]
pub struct PlayerUse {
	pub driver: Entity,
}
