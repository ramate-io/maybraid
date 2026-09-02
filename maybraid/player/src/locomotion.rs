//! Wish facing and walk/run/jump clips.

use avian3d::prelude::LinearVelocity;
use bevy::ecs::query::Has;
use bevy::prelude::*;
use crozon_characters::{
	AnimClip, AnimRef, AnimRefRoot, CharacterMembers, CharacterRig, CharacterRigRole, CharacterRoot,
};

use crate::body::{CharacterController, Jumping, MoveWish};
use crate::identity::{PlayerVisual, PlayerYawOwner};

const WALK_SPEED: f32 = 1.0;
const RUN_SPEED: f32 = 5.0;
const FACE_DEADZONE: f32 = 0.05;
const TURN_RATE: f32 = 5.5;

pub(crate) fn face_wish_yaw(
	time: Res<Time>,
	wishes: Query<&MoveWish, With<CharacterController>>,
	mut visuals: Query<(&mut Transform, Option<&PlayerYawOwner>, &ChildOf), With<PlayerVisual>>,
) {
	for (mut visual, owner, child_of) in &mut visuals {
		if owner.copied().unwrap_or_default() != PlayerYawOwner::Wish {
			continue;
		}
		let Ok(wish) = wishes.get(child_of.parent()) else {
			continue;
		};
		face_wish(&mut visual, wish.0, time.delta_secs());
	}
}

fn face_wish(visual: &mut Transform, wish: Vec3, dt: f32) {
	let target = Vec3::new(wish.x, 0.0, wish.z);
	if target.length_squared() < 1e-4 {
		return;
	}
	let target = target.normalize();
	let current = {
		let facing = -visual.forward();
		let xz = Vec3::new(facing.x, 0.0, facing.z);
		if xz.length_squared() < 1e-4 {
			visual.look_to(-target, Vec3::Y);
			return;
		}
		xz.normalize()
	};
	let angle = current.angle_between(target);
	if angle < FACE_DEADZONE {
		return;
	}
	let t = (TURN_RATE * dt / angle).min(1.0);
	visual.look_to(-current.slerp(target, t), Vec3::Y);
}

pub fn drive_player_locomotion(
	mut commands: Commands,
	controllers: Query<(&LinearVelocity, &MoveWish, Has<Jumping>), With<CharacterController>>,
	visuals: Query<(&CharacterMembers, &ChildOf), (With<PlayerVisual>, With<CharacterRoot>)>,
	rigs: Query<&CharacterRig>,
	anims: Query<&AnimRefRoot>,
) {
	for (members, child_of) in &visuals {
		let Ok((velocity, _wish, jumping)) = controllers.get(child_of.parent()) else {
			continue;
		};
		let speed = Vec3::new(velocity.x, 0.0, velocity.z).length();
		for member in members.iter() {
			let Ok(rig) = rigs.get(member) else {
				continue;
			};
			if rig.role != CharacterRigRole::Body {
				continue;
			}
			let clip = if jumping && speed > RUN_SPEED {
				AnimClip::leap()
			} else if jumping {
				AnimClip::jump()
			} else if speed > RUN_SPEED {
				AnimClip::run()
			} else if speed > WALK_SPEED {
				AnimClip::walk()
			} else {
				AnimClip::still()
			};
			let desired = AnimRef::new(clip);
			let needs = match anims.get(member) {
				Ok(root) => root.0 != desired,
				Err(_) => true,
			};
			if needs {
				commands.entity(member).insert(AnimRefRoot(desired));
			}
		}
	}
}
