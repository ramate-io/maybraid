//! Digital pad buttons and this-frame edges.

use bevy::prelude::*;

/// Xbox-letter virtual face. Nintendo layouts are remapped in the gamepad producer
/// (Bevy `South` → [`PadButton::A`], `East` → [`PadButton::B`], …).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum PadButton {
	A = 0,
	B,
	X,
	Y,
	BumperFocus,
	BumperFire,
	StickClickMove,
	StickClickLook,
	DpadUp,
	DpadDown,
	DpadLeft,
	DpadRight,
	Start,
	Select,
	/// Digital view of [`crate::VirtualPad::trigger_focus`] after the press threshold.
	TriggerFocus,
	/// Digital view of [`crate::VirtualPad::trigger_fire`] after the press threshold.
	TriggerFire,
}

/// Number of [`PadButton`] variants. Index matches [`PadButton`] discriminants.
pub const PAD_BUTTON_COUNT: usize = 16;

impl PadButton {
	pub const ALL: [Self; PAD_BUTTON_COUNT] = [
		Self::A,
		Self::B,
		Self::X,
		Self::Y,
		Self::BumperFocus,
		Self::BumperFire,
		Self::StickClickMove,
		Self::StickClickLook,
		Self::DpadUp,
		Self::DpadDown,
		Self::DpadLeft,
		Self::DpadRight,
		Self::Start,
		Self::Select,
		Self::TriggerFocus,
		Self::TriggerFire,
	];

	pub fn index(self) -> usize {
		self as usize
	}

	/// Position-normalized Bevy button → Xbox-letter pad. Analog triggers are not
	/// mapped here; they become [`PadButton::TriggerFocus`] / [`PadButton::TriggerFire`]
	/// after the analog threshold.
	pub fn from_gamepad(button: GamepadButton) -> Option<Self> {
		Some(match button {
			GamepadButton::South => Self::A,
			GamepadButton::East => Self::B,
			GamepadButton::West => Self::X,
			GamepadButton::North => Self::Y,
			GamepadButton::LeftTrigger => Self::BumperFocus,
			GamepadButton::RightTrigger => Self::BumperFire,
			GamepadButton::LeftThumb => Self::StickClickMove,
			GamepadButton::RightThumb => Self::StickClickLook,
			GamepadButton::DPadUp => Self::DpadUp,
			GamepadButton::DPadDown => Self::DpadDown,
			GamepadButton::DPadLeft => Self::DpadLeft,
			GamepadButton::DPadRight => Self::DpadRight,
			GamepadButton::Start => Self::Start,
			GamepadButton::Select => Self::Select,
			_ => return None,
		})
	}
}

/// Rising or falling edge. Hold is [`ButtonInput::pressed`], not a third phase.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ButtonPhase {
	Pressed,
	Released,
}

impl ButtonPhase {
	pub fn from_button_state(state: bevy::input::ButtonState) -> Self {
		match state {
			bevy::input::ButtonState::Pressed => Self::Pressed,
			bevy::input::ButtonState::Released => Self::Released,
		}
	}
}

/// One digital edge this frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ButtonStroke<T> {
	pub button: T,
	pub phase: ButtonPhase,
	/// OS auto-repeat. Gameplay should ignore this; text / backspace may not.
	pub repeat: bool,
}

impl<T> ButtonStroke<T> {
	pub fn pressed(button: T) -> Self {
		Self { button, phase: ButtonPhase::Pressed, repeat: false }
	}

	pub fn released(button: T) -> Self {
		Self { button, phase: ButtonPhase::Released, repeat: false }
	}

	pub fn with_repeat(mut self, repeat: bool) -> Self {
		self.repeat = repeat;
		self
	}
}
