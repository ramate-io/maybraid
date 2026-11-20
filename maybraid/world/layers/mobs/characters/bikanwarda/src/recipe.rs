//! Inner [`CharacterComponents`] recipe for Bikanwarda.

use bevy::prelude::Component;
use bevy::shape::Aabb3d;
use characters::{
	components::{CharacterComponents, LocomotionCapsule},
	layer::Layers,
	nodes::{PartNode, RigNode},
	species::epiphant::recipe::Epiphant,
	CharacterRecipe,
};
use lod::scene::LodSceneLevel;

use crate::presentation::BikanwardaPresentation;
use crate::spec::{BikanwardaForm, BikanwardaSpec};

/// Semantic Bikanwarda data on the assembly root.
#[derive(Component, Clone, PartialEq)]
pub struct BikanwardaActor {
	pub epiphant: Epiphant,
	pub form: BikanwardaForm,
	pub presentation: BikanwardaPresentation,
	pub uniform_scale: f32,
}

impl Default for BikanwardaActor {
	fn default() -> Self {
		BikanwardaActorRecipe::default().components()
	}
}

/// Config → components for all Bikanwarda forms.
#[derive(Debug, Clone, PartialEq)]
pub struct BikanwardaActorRecipe {
	pub spec: BikanwardaSpec,
}

impl Default for BikanwardaActorRecipe {
	fn default() -> Self {
		Self::for_form(BikanwardaForm::Encounter)
	}
}

impl BikanwardaActorRecipe {
	pub fn for_form(form: BikanwardaForm) -> Self {
		let spec = match form {
			BikanwardaForm::Encounter => BikanwardaSpec::encounter(),
			BikanwardaForm::Ward => BikanwardaSpec::ward(),
			BikanwardaForm::Enraged => BikanwardaSpec::enraged(),
		};
		Self { spec }
	}

	pub fn from_spec(spec: BikanwardaSpec) -> Self {
		Self { spec }
	}

	pub fn uniform_scale(&self) -> f32 {
		self.spec.uniform_scale()
	}

	pub fn locomotion_capsule(&self) -> LocomotionCapsule {
		self.components().locomotion_capsule()
	}
}

impl CharacterRecipe for BikanwardaActorRecipe {
	type Components = BikanwardaActor;

	fn components(&self) -> Self::Components {
		let mut epiphant = Epiphant::from_config(&self.spec.epiphant);
		epiphant.colors = self.spec.presentation.colors.clone();
		BikanwardaActor {
			epiphant,
			form: self.spec.form,
			presentation: self.spec.presentation.clone(),
			uniform_scale: self.uniform_scale(),
		}
	}

	fn clothing_layers(&self) -> Vec<characters::ClothingLayer> {
		Vec::new()
	}
}

impl BikanwardaActor {
	/// Motor hull after encounter uniform scale (matches root [`Transform::scale`]).
	pub fn scaled_locomotion_capsule(&self) -> LocomotionCapsule {
		self.epiphant.locomotion_capsule().scaled(self.uniform_scale)
	}
}

impl CharacterComponents for BikanwardaActor {
	fn locomotion_capsule(&self) -> LocomotionCapsule {
		self.epiphant.locomotion_capsule()
	}

	fn rig_nodes_for_level(&self, level: LodSceneLevel) -> Layers<RigNode> {
		self.epiphant.rig_nodes_for_level(level)
	}

	fn part_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<PartNode> {
		use crate::textures::{material_ref_for_slot, EpiphantTextureSlot};
		use characters::assets::AssetNormalization;

		// Painted #1589 export is already assembled in character space. Do not
		// socket or skin it — that would double-apply rest placement. Rigs stay
		// on [`Self::rig_nodes_for_level`] for colliders, sockets, and Bray origin.
		let part = |slot: EpiphantTextureSlot| {
			let mut node = PartNode::glb(
				slot.part_slot(),
				slot.object_name(),
				slot.preview_scene_path(),
				AssetNormalization::IDENTITY,
			);
			if let Some(material) = material_ref_for_slot(slot.part_slot(), &self.presentation) {
				node = node.with_material(material);
			}
			node
		};
		let mut out = Layers::from_labeled(
			"body",
			vec![part(EpiphantTextureSlot::Body), part(EpiphantTextureSlot::Tail)],
		);
		out.extend_labeled("head", vec![part(EpiphantTextureSlot::Head)]);
		out.extend_labeled(
			"features",
			vec![
				part(EpiphantTextureSlot::EyeLeft),
				part(EpiphantTextureSlot::EyeRight),
				part(EpiphantTextureSlot::Nose),
				part(EpiphantTextureSlot::EarLeft),
				part(EpiphantTextureSlot::EarRight),
			],
		);
		out
	}

	fn preview_aabb(&self) -> Aabb3d {
		self.scaled_locomotion_capsule().preview_aabb()
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use character_animations::animations::MOUTH_EMIT_SOCKET;
	use characters::assembly::CharacterPartSlot;
	use lod::scene::LodSceneLevel;

	#[test]
	fn bikanwarda_keeps_head_rig_for_bray_while_visual_nose_is_unskinned() {
		let actor = BikanwardaActorRecipe::default().components();
		let parts = actor.part_nodes_for_level(LodSceneLevel::High).flatten();
		let nose = parts
			.iter()
			.find(|part| part.slot == CharacterPartSlot::Nose)
			.expect("epiphant nose part");
		assert!(nose.socket.is_none(), "assembled export must not re-socket");
		assert!(nose.skin.is_none(), "assembled export is unskinned");
		let rigs = actor.rig_nodes_for_level(LodSceneLevel::High).flatten();
		assert!(
			rigs.iter().any(|rig| rig.id == characters::socket::RigId::Head),
			"pronograde head rig still supplies {MOUTH_EMIT_SOCKET} for Bray origin"
		);
	}
}
