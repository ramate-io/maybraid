//! Connected-pad rumble. Mouse and keyboard do not vibrate.

use std::time::Duration;

use bevy::input::gamepad::{GamepadRumbleIntensity, GamepadRumbleRequest};
use bevy::prelude::*;

/// Gameplay rumble pulse. [`fan_out_pad_rumble`] copies it onto every live
/// [`Gamepad`]. Linux / Windows play that through gilrs; Apple plays it through
/// GameController haptics.
#[derive(Message, Clone, Copy, Debug, PartialEq)]
pub struct PadRumble {
	pub duration: Duration,
	pub intensity: GamepadRumbleIntensity,
}

impl PadRumble {
	pub fn motors(duration: Duration, weak: f32, strong: f32) -> Self {
		Self {
			duration,
			intensity: GamepadRumbleIntensity {
				weak_motor: weak.clamp(0.0, 1.0),
				strong_motor: strong.clamp(0.0, 1.0),
			},
		}
	}

	pub fn is_silent(self) -> bool {
		self.duration.is_zero()
			|| (self.intensity.weak_motor <= 0.0 && self.intensity.strong_motor <= 0.0)
	}
}

#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PadRumbleSystems {
	FanOut,
	Play,
}

pub(crate) fn fan_out_pad_rumble(
	mut pulses: MessageReader<PadRumble>,
	pads: Query<Entity, With<Gamepad>>,
	mut rumble: MessageWriter<GamepadRumbleRequest>,
) {
	for pulse in pulses.read() {
		if pulse.is_silent() {
			warn!("pad_rumble: skipped silent pulse");
			continue;
		}
		let n = pads.iter().count();
		if n == 0 {
			warn!(
				"pad_rumble: fan_out had pulse ({:?} weak={:.2} strong={:.2}) but 0 Gamepad entities",
				pulse.duration, pulse.intensity.weak_motor, pulse.intensity.strong_motor
			);
			continue;
		}
		info!(
			"pad_rumble: fan_out pads={n} duration={:?} weak={:.2} strong={:.2}",
			pulse.duration, pulse.intensity.weak_motor, pulse.intensity.strong_motor
		);
		for gamepad in &pads {
			rumble.write(GamepadRumbleRequest::Add {
				gamepad,
				duration: pulse.duration,
				intensity: pulse.intensity,
			});
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn motors_clamp_to_unit_interval() {
		let pulse = PadRumble::motors(Duration::from_millis(40), 1.5, -0.2);
		assert!((pulse.intensity.weak_motor - 1.0).abs() < 1e-5);
		assert!(pulse.intensity.strong_motor.abs() < 1e-5);
		assert!(!pulse.is_silent());
	}

	#[test]
	fn zero_duration_or_intensity_is_silent() {
		assert!(PadRumble::motors(Duration::ZERO, 0.4, 0.2).is_silent());
		assert!(PadRumble::motors(Duration::from_millis(40), 0.0, 0.0).is_silent());
	}

	#[test]
	fn fan_out_copies_onto_each_gamepad() {
		let mut app = App::new();
		app.add_plugins(MinimalPlugins)
			.add_message::<PadRumble>()
			.add_message::<GamepadRumbleRequest>()
			.add_systems(Update, fan_out_pad_rumble);
		let left = app.world_mut().spawn(Gamepad::default()).id();
		let right = app.world_mut().spawn(Gamepad::default()).id();
		app.world_mut()
			.write_message(PadRumble::motors(Duration::from_millis(40), 0.3, 0.1));
		app.update();

		let messages = app.world().resource::<Messages<GamepadRumbleRequest>>();
		let mut cursor = messages.get_cursor();
		let got: Vec<_> = cursor.read(messages).cloned().collect();
		assert_eq!(got.len(), 2);
		let entities: Vec<Entity> = got.iter().map(GamepadRumbleRequest::gamepad).collect();
		assert!(entities.contains(&left));
		assert!(entities.contains(&right));
		for request in &got {
			let GamepadRumbleRequest::Add { duration, intensity, .. } = request else {
				panic!("expected Add");
			};
			assert_eq!(*duration, Duration::from_millis(40));
			assert!((intensity.weak_motor - 0.3).abs() < 1e-5);
			assert!((intensity.strong_motor - 0.1).abs() < 1e-5);
		}
	}

	#[test]
	fn silent_pulse_does_not_fan_out() {
		let mut app = App::new();
		app.add_plugins(MinimalPlugins)
			.add_message::<PadRumble>()
			.add_message::<GamepadRumbleRequest>()
			.add_systems(Update, fan_out_pad_rumble);
		app.world_mut().spawn(Gamepad::default());
		app.world_mut()
			.write_message(PadRumble::motors(Duration::from_millis(40), 0.0, 0.0));
		app.update();

		let messages = app.world().resource::<Messages<GamepadRumbleRequest>>();
		let mut cursor = messages.get_cursor();
		assert_eq!(cursor.read(messages).count(), 0);
	}

	#[test]
	fn no_gamepad_means_no_rumble() {
		let mut app = App::new();
		app.add_plugins(MinimalPlugins)
			.add_message::<PadRumble>()
			.add_message::<GamepadRumbleRequest>()
			.add_systems(Update, fan_out_pad_rumble);
		app.world_mut()
			.write_message(PadRumble::motors(Duration::from_millis(40), 0.4, 0.2));
		app.update();

		let messages = app.world().resource::<Messages<GamepadRumbleRequest>>();
		let mut cursor = messages.get_cursor();
		assert_eq!(cursor.read(messages).count(), 0);
	}
}
