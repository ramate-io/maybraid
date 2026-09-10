//! Stick motion while a [`SkillMapUser`] holds the bumper chord.

use bevy::prelude::*;
use maybraid_character_controller::CharacterIntent;

use crate::user::{SkillMapHeld, SkillMapMember, SkillMapSteerLock, SkillMapUser};
use crate::viewport::SkillMapViewportCamera;
use crate::SkillMapEnabled;

pub const CURSOR_SPEED: f32 = 64.0;
pub const WATER_LOCK_SECS: f32 = 2.0;

#[derive(Component)]
pub struct SkillMapCursor;

pub fn apply_skill_map_intents(
	enabled: Res<SkillMapEnabled>,
	mut intents: MessageReader<CharacterIntent>,
	mut users: Query<&mut SkillMapHeld, With<SkillMapUser>>,
) {
	let mut mapped = false;
	for intent in intents.read() {
		if matches!(*intent, CharacterIntent::SkillMap) {
			mapped = true;
		}
	}
	for mut held in &mut users {
		held.0 = enabled.0 && mapped;
	}
}

pub fn tick_steer_lock(
	time: Res<Time>,
	mut locks: Query<&mut SkillMapSteerLock, With<SkillMapUser>>,
) {
	let dt = time.delta_secs();
	for mut lock in &mut locks {
		if lock.remaining > 0.0 {
			lock.remaining = (lock.remaining - dt).max(0.0);
		}
	}
}

pub fn steer_cursors(
	time: Res<Time>,
	enabled: Res<SkillMapEnabled>,
	mut intents: MessageReader<CharacterIntent>,
	users: Query<(&SkillMapUser, &SkillMapHeld, &SkillMapSteerLock)>,
	mut cameras: Query<(&SkillMapMember, &mut Transform), With<SkillMapViewportCamera>>,
) {
	if !enabled.0 {
		for _ in intents.read() {}
		return;
	}
	let mut move_stick = Vec2::ZERO;
	for intent in intents.read() {
		if let CharacterIntent::Move(value) = *intent {
			move_stick = value;
		}
	}
	if move_stick == Vec2::ZERO {
		return;
	}
	let dt = time.delta_secs();
	for (member, mut transform) in &mut cameras {
		let Some((user, held, lock)) =
			users.iter().find(|(user, _, _)| user.maps == member.session)
		else {
			continue;
		};
		if !held.0 || lock.locked() {
			continue;
		}
		let delta = move_stick * user.settings.cursor_speed * dt;
		transform.translation.x += delta.x;
		transform.translation.y += delta.y;
	}
}
