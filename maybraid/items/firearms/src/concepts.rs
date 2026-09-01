//! Named firearm kits assembled from [`firearms_components`] nodes.

use firearms_components::assets::guns;
use firearms_components::{FirearmComponents, Layers, PartNode};
use lod::gen::LodSceneLevel;

/// Authored firearm concepts currently in `maybraid/art/items/guns/`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, bevy::prelude::Component)]
pub enum FirearmConcept {
	/// Receiver + barrel + grip, socketed onto `barrel` / `grip` bones.
	#[default]
	Bullpup,
	Silopup,
	Keelripe,
	Reltor,
	Samsonist,
	Snailer,
	/// Barrel-only concept (no receiver kit yet).
	Laznard,
}

impl FirearmConcept {
	pub fn label(self) -> &'static str {
		match self {
			Self::Bullpup => "bullpup",
			Self::Silopup => "silopup",
			Self::Keelripe => "keelripe",
			Self::Reltor => "reltor",
			Self::Samsonist => "samsonist",
			Self::Snailer => "snailer",
			Self::Laznard => "laznard",
		}
	}

	/// Baked one-mesh concept, when the kit was also exported as a single GLB.
	pub fn baked_concept(self) -> Option<firearms_components::AssetPath> {
		match self {
			Self::Bullpup => Some(guns::BULLPUP_FULL_CONCEPT),
			Self::Silopup => Some(guns::SILOPUP_FULL_CONCEPT),
			_ => None,
		}
	}
}

impl FirearmComponents for FirearmConcept {
	fn body_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<PartNode> {
		let node = match self {
			Self::Bullpup => PartNode::body("bullpup-body", guns::BULLPUP_BODY.as_str()),
			Self::Silopup => PartNode::body("silopup-body", guns::SILOPUP_BODY.as_str()),
			Self::Keelripe => PartNode::body("keelripe-body", guns::KEELRIPE_BODY.as_str()),
			Self::Reltor => PartNode::body("reltor-body", guns::RELTOR_BODY.as_str()),
			Self::Samsonist => PartNode::body("samsonist-body", guns::SAMSONIST_BODY.as_str()),
			Self::Snailer => PartNode::body("snailer-body", guns::SNAILER_BODY.as_str()),
			Self::Laznard => return Layers::new(),
		};
		Layers::from_labeled("body", vec![node])
	}

	fn barrel_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<PartNode> {
		let node = match self {
			Self::Bullpup => PartNode::barrel("bullpup-barrel", guns::BULLPUP_BARREL.as_str()),
			Self::Laznard => PartNode::barrel("laznard-barrel", guns::LAZNARD_BARREL.as_str()),
			_ => return Layers::new(),
		};
		Layers::from_labeled("barrel", vec![node])
	}

	fn grip_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<PartNode> {
		match self {
			Self::Bullpup => Layers::from_labeled(
				"grip",
				vec![PartNode::grip("bullpup-grip", guns::BULLPUP_GRIP.as_str())],
			),
			_ => Layers::new(),
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use firearms_components::{FirearmPartSlot, SocketRef};

	#[test]
	fn bullpup_assembles_body_barrel_grip() {
		let gun = FirearmConcept::Bullpup;
		let bodies = gun.body_nodes_for_level(LodSceneLevel::High).flatten();
		let barrels = gun.barrel_nodes_for_level(LodSceneLevel::High).flatten();
		let grips = gun.grip_nodes_for_level(LodSceneLevel::High).flatten();
		assert_eq!(bodies[0].slot, FirearmPartSlot::Body);
		assert_eq!(barrels[0].slot, FirearmPartSlot::Barrel);
		assert_eq!(grips[0].slot, FirearmPartSlot::Grip);
		assert_eq!(barrels[0].socket, Some(SocketRef::on("barrel")));
		assert_eq!(grips[0].socket, Some(SocketRef::on("grip")));
		assert!(bodies[0].socket.is_none());
		assert!(gun.rig_nodes_for_level(LodSceneLevel::High).is_empty());
	}

	#[test]
	fn body_only_concepts_have_no_kit_attachments() {
		for gun in [
			FirearmConcept::Silopup,
			FirearmConcept::Keelripe,
			FirearmConcept::Reltor,
			FirearmConcept::Samsonist,
			FirearmConcept::Snailer,
		] {
			assert_eq!(gun.body_nodes_for_level(LodSceneLevel::High).len(), 1);
			assert!(gun.barrel_nodes_for_level(LodSceneLevel::High).is_empty());
			assert!(gun.grip_nodes_for_level(LodSceneLevel::High).is_empty());
		}
	}

	#[test]
	fn laznard_is_barrel_only() {
		let gun = FirearmConcept::Laznard;
		assert!(gun.body_nodes_for_level(LodSceneLevel::High).is_empty());
		assert_eq!(gun.barrel_nodes_for_level(LodSceneLevel::High).len(), 1);
	}
}
