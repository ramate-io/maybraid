pub mod animations;
pub mod rigs;

use bevy::prelude::*;

/// Side effects from an animation pass, applied outside the bone pose (e.g. armature root).
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Effects {
	/// Offset relative to the armature bind transform.
	pub r#move: Option<Transform>,
}

/// Normalized animation sampling coordinate.
///
/// Cyclic animations wrap via [`Self::cycle`]; one-shot animations clamp via [`Self::clamp`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Progress(pub f32);

impl Progress {
	pub fn new(value: f32) -> Self {
		Self(value)
	}

	/// Cyclic progress in `[0.0, 1.0)`.
	pub fn cycle(self) -> f32 {
		self.0.rem_euclid(1.0)
	}

	/// Clamped one-shot progress in `[0.0, 1.0]`.
	pub fn clamp(self) -> f32 {
		self.0.clamp(0.0, 1.0)
	}

	/// Whole completed cycles.
	pub fn cycles(self) -> f32 {
		self.0.floor()
	}

	/// True once a one-shot animation has reached or passed its end.
	pub fn is_complete(self) -> bool {
		self.0 >= 1.0
	}
}

/// Applies an animation to a rig at a specific progress value.
///
/// For cyclic animations, progress wraps by taking `progress.fract()`. Values above `1.0`
/// are valid (e.g. `1.5` samples halfway through the next cycle). One-shot animations
/// clamp progress to `[0.0, 1.0]`.
pub trait Animation<Rig> {
	fn apply(&self, rig: &mut Rig, progress: f32) -> Effects;
}

/// Animation playback state owned by the controller.
#[derive(Debug, Clone)]
pub struct Playing<A> {
	pub animation: A,
	pub progress: f32,
	pub speed: f32,
}

impl<A> Playing<A> {
	pub fn new(animation: A, speed: f32) -> Self {
		Self { animation, progress: 0.0, speed }
	}

	pub fn advance(&mut self, delta_seconds: f32) {
		self.progress += delta_seconds * self.speed;
	}

	pub fn apply<R>(&self, rig: &mut R) -> Effects
	where
		A: Animation<R>,
	{
		self.animation.apply(rig, self.progress)
	}
}
