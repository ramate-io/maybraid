use super::{Bone, BoneTable, Name};

use bevy::prelude::*;

/// The common slider type.
///
/// The name is mostly for standardization to display purposes.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Slider {
	pub name: String,
	pub value: f32,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SliderBoneEffect {
	pub bone_name: Name,
	pub bone_transform: Transform,
}

/// A type that can be slid.
pub trait Slidable {
	/// The slider that controls this slidable.
	fn slider(&self) -> Slider;

	/// The bones that are affected by this slidable.
	///
	/// When applying the slider to a rig, the bone effects are used to determine
	/// the new transform of the bones.
	fn slider_bone_effects(&self) -> Vec<SliderBoneEffect>;
}

/// Gives a list of sliders for sizing the rig.
///
/// This is most used during character creation.
pub trait Sliders<S: Slidable> {
	fn list_sliders(&self) -> Vec<S>;
}

impl BoneTable {
	pub fn apply_sliders<S: Slidable>(&mut self, sliders: &[S]) {
		for slider in sliders {
			let bone_effects = slider.slider_bone_effects();
			for bone_effect in bone_effects {
				let bone = self.get_mut(&bone_effect.bone_name);
				if let Some(bone) = bone {
					// Not sure this should be * here.
					// We should probably just set the bone.
					bone.transform = bone_effect.bone_transform;
				}
			}
		}
	}
}
