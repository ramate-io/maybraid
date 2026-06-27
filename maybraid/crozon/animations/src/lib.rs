pub mod animations;
pub mod poses;
pub mod rigs;

use bevy::prelude::*;

/// Side effects from an animation pass, applied outside the bone pose (e.g. armature root).
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Effects {
	/// Offset relative to the armature bind transform.
	pub r#move: Option<Transform>,
}

pub trait Animation<Rig> {
	fn apply(&self, rig: &mut Rig) -> Effects;
}
