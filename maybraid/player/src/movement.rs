//! Footsteps on grounded walk / run, cadence-matched to the locomotion clips.

use avian3d::prelude::LinearVelocity;
use bevy::prelude::*;
use crozon_characters::AnimId;
use maybraid_audio::{Audio, AudioClip, AudioSystems, Mixer, MovementSounds, MovementState};

use crate::body::{CharacterController, Grounded, Jumping, LEAP_SPEED};
use crate::locomotion::WALK_SPEED;
use crate::stance::{CharacterStance, StanceKind};
use crate::{Npc, Player};

pub(crate) fn ensure_movement_states(
	mut commands: Commands,
	bodies: Query<
		Entity,
		(With<CharacterController>, Without<MovementState>, Or<(With<Player>, With<Npc>)>),
	>,
) {
	for entity in &bodies {
		commands.entity(entity).insert(MovementState::seeded(entity.to_bits()));
	}
}

pub(crate) fn play_footsteps(
	mut commands: Commands,
	time: Res<Time>,
	mut bodies: Query<
		(
			Entity,
			&GlobalTransform,
			&LinearVelocity,
			&mut MovementState,
			Option<&Grounded>,
			Option<&Jumping>,
			Option<&CharacterStance>,
		),
		With<CharacterController>,
	>,
	clips: Res<Assets<AudioClip>>,
	audio: Option<Res<Audio>>,
	sounds: Option<Res<MovementSounds>>,
	mixer: Option<ResMut<Mixer>>,
	listeners: Query<&GlobalTransform, With<SpatialListener>>,
) {
	let (Some(audio), Some(sounds), Some(mut mixer), Some(listener)) =
		(audio.as_deref(), sounds.as_deref(), mixer, listeners.iter().next())
	else {
		return;
	};
	let dt = time.delta_secs();
	for (_entity, transform, velocity, mut gait, grounded, jumping, stance) in &mut bodies {
		let speed = Vec3::new(velocity.x, 0.0, velocity.z).length();
		let rate = footstep_rate(grounded.is_some(), jumping, stance, speed);
		let steps = gait.take_steps(rate, dt);
		if steps == 0 {
			continue;
		}
		let point = transform.translation();
		for _ in 0..steps {
			sounds.play_footstep(
				&mut commands,
				&mut gait,
				&clips,
				audio,
				&mut mixer,
				listener,
				point,
			);
		}
	}
}

/// Two plants per walk/run cycle. Air, stance, and idle stay quiet.
pub(crate) fn footstep_rate(
	grounded: bool,
	jumping: Option<&Jumping>,
	stance: Option<&CharacterStance>,
	speed: f32,
) -> f32 {
	if !grounded || jumping.is_some() {
		return 0.0;
	}
	let kind = stance.map(|stance| stance.kind).unwrap_or(StanceKind::Stand);
	if !matches!(kind, StanceKind::Stand) {
		return 0.0;
	}
	if speed > LEAP_SPEED {
		AnimId::Run.default_speed() * 2.0
	} else if speed > WALK_SPEED {
		AnimId::Walk.default_speed() * 2.0
	} else {
		0.0
	}
}

pub(crate) fn configure_movement(app: &mut App) {
	app.add_systems(
		PostUpdate,
		(ensure_movement_states, play_footsteps)
			.chain()
			.after(TransformSystems::Propagate)
			.before(AudioSystems::Sync),
	);
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn stand_walk_uses_two_plants_per_cycle() {
		assert!(
			(footstep_rate(true, None, None, 3.0) - AnimId::Walk.default_speed() * 2.0).abs()
				< 1e-5
		);
		assert!(
			(footstep_rate(true, None, None, 9.0) - AnimId::Run.default_speed() * 2.0).abs() < 1e-5
		);
	}

	#[test]
	fn idle_air_and_stance_stay_quiet() {
		assert_eq!(footstep_rate(true, None, None, 0.4), 0.0);
		assert_eq!(footstep_rate(false, None, None, 6.0), 0.0);
		let jump = Jumping::start(0.0);
		assert_eq!(footstep_rate(true, Some(&jump), None, 6.0), 0.0);
		let squat = CharacterStance { kind: StanceKind::Squat, blend: 1.0 };
		assert_eq!(footstep_rate(true, None, Some(&squat), 6.0), 0.0);
	}

	#[test]
	fn player_controller_gets_a_gait() {
		let mut app = App::new();
		app.add_systems(Update, ensure_movement_states);
		let player = app.world_mut().spawn((Player, CharacterController)).id();
		app.update();
		assert!(app.world().get::<MovementState>(player).is_some());
	}
}
