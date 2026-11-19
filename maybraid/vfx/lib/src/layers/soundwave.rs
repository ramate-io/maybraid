//! Standalone directional soundwave layer.

use bevy::prelude::*;

use crate::composition::{EffectDefinition, EffectLayer, SoundwavePart};
use crate::names::SOUNDWAVE;
use crate::parts::soundwave::soundwave_torus_mesh;

pub const SOUNDWAVE_DURATION: f32 = 1.6;

pub fn compile_mesh(mesh: Handle<Mesh>) -> SoundwavePart {
	SoundwavePart { name: SOUNDWAVE.into(), mesh, duration: SOUNDWAVE_DURATION }
}

/// Pose the Y-axle torus so the disc faces a horizontal Bray heading (−Z).
pub fn facing_player_transform() -> Transform {
	Transform {
		translation: Vec3::ZERO,
		rotation: Quat::from_rotation_arc(Vec3::Y, Vec3::NEG_Z),
		scale: Vec3::ONE,
	}
}

pub fn definition(part: SoundwavePart) -> EffectDefinition {
	EffectDefinition::new(
		SOUNDWAVE,
		[EffectLayer::soundwave(part).with_transform(facing_player_transform())],
	)
}

pub fn torus_mesh() -> Mesh {
	soundwave_torus_mesh()
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::composition::EffectPart;

	#[test]
	fn layer_owns_the_soundwave_part() {
		let part = compile_mesh(Handle::default());
		let def = definition(part);
		assert_eq!(def.name, SOUNDWAVE);
		assert!(matches!(def.layers[0].part, EffectPart::Soundwave(_)));
	}

	#[test]
	fn recipe_faces_a_horizontal_heading() {
		let axle = facing_player_transform().rotation * Vec3::Y;
		assert!(
			(axle - Vec3::NEG_Z).length() < 1e-4,
			"playground recipe must face the camera like Bray, got {axle}"
		);
	}
}
