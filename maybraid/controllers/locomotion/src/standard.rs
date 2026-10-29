//! Addressed semantic frames → actor-local Standard motor requests.

use bevy::prelude::*;
use maybraid_character_controller::{
	ActorControlStatus, CharacterIntent, CharacterIntentFrame, OrdinaryLocomotion,
};
use player::{
	CharacterController, MotorIntent, MotorSample, Player, PlayerLook, SpeedCapRequest,
	SprintRequest, StanceRequest,
};

const FOCUS_EPS: f32 = 1e-6;

/// Consume only actor-owned addressed frames; final writes revalidate their original binding.
pub fn interpret_standard_locomotion(
	mut commands: Commands,
	mut actors: Query<
		(
			&PlayerLook,
			&mut MotorIntent,
			Option<&OrdinaryLocomotion>,
			Option<&CharacterIntentFrame>,
			Option<&ActorControlStatus>,
		),
		(With<CharacterController>, With<Player>),
	>,
) {
	for (look, mut motor, ordinary, frame, status) in &mut actors {
		if status.is_some_and(|status| status.is_frozen()) {
			// Preserve holds every suspended tick while preventing new translation or edges.
			motor.set_standard(MotorSample::default());
			continue;
		}
		let Some(input) = frame.and_then(CharacterIntentFrame::get) else {
			continue;
		};
		let sample = StandardControl::from_intents(input.payload())
			.sample(look, ordinary.is_none_or(|enabled| enabled.0));
		input.clone().map(|_| sample).queue(&mut commands, |world, input| {
			let Some(entity) = input.target().actor_entity() else {
				return;
			};
			if let Some(mut motor) = world.get_mut::<MotorIntent>(entity) {
				motor.set_standard(input.into_payload());
			}
		});
	}
}

#[derive(Default)]
struct StandardControl {
	move_stick: Vec2,
	jump: bool,
	focusing: bool,
	start_sprint: bool,
	stop_sprint: bool,
	change_squat: bool,
	change_prone: bool,
}

impl StandardControl {
	fn from_intents(intents: &[CharacterIntent]) -> Self {
		let mut input = Self::default();
		for intent in intents {
			match *intent {
				CharacterIntent::Move(value) => input.move_stick = value,
				CharacterIntent::Jump => input.jump = true,
				CharacterIntent::Focus(value) if value > FOCUS_EPS => input.focusing = true,
				CharacterIntent::StartSprint => input.start_sprint = true,
				CharacterIntent::StopSprint => input.stop_sprint = true,
				CharacterIntent::ChangeSquat => input.change_squat = true,
				CharacterIntent::ChangeProne => input.change_prone = true,
				_ => {}
			}
		}
		input
	}

	fn sample(self, look: &PlayerLook, ordinary: bool) -> MotorSample {
		let sprint = if self.focusing {
			SprintRequest::Release
		} else if ordinary && self.start_sprint {
			SprintRequest::Start
		} else if ordinary && self.stop_sprint {
			SprintRequest::Stop
		} else {
			SprintRequest::Neutral
		};
		let stance = if ordinary && self.start_sprint {
			StanceRequest::Stand
		} else if ordinary && self.change_prone {
			StanceRequest::ChangeProne
		} else if ordinary && self.change_squat {
			StanceRequest::ChangeSquat
		} else {
			StanceRequest::Neutral
		};
		MotorSample {
			translation: if ordinary { look.wish_from_stick(self.move_stick) } else { Vec3::ZERO },
			jump: ordinary && self.jump,
			sprint,
			stance,
			speed_cap: SpeedCapRequest::Release,
		}
	}
}
