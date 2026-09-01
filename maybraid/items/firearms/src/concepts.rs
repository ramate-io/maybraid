//! Named firearm kits assembled from [`firearms_components`] nodes.

use firearms_components::assets::guns;
use firearms_components::{FirearmComponents, Layers, PartNode, RigNode};
use lod::gen::LodSceneLevel;

/// Authored firearm concepts currently in `maybraid/art/items/guns/`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, bevy::prelude::Component)]
pub enum FirearmConcept {
	/// Receiver + barrel + grip, socketed onto `body` / `barrel` / `grip` bones.
	#[default]
	Bullpup,
	Silopup,
	Keelripe,
	Reltor,
	Samsonist,
	Snailer,
	/// Barrel-only concept (sits on the shared receiver `barrel` bone).
	Laznard,
}

impl FirearmConcept {
	pub const ALL: [Self; 7] = [
		Self::Bullpup,
		Self::Silopup,
		Self::Keelripe,
		Self::Reltor,
		Self::Samsonist,
		Self::Snailer,
		Self::Laznard,
	];

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

impl std::str::FromStr for FirearmConcept {
	type Err = String;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		Self::ALL
			.iter()
			.copied()
			.find(|concept| concept.label() == s)
			.ok_or_else(|| format!("unknown firearm concept {s:?}"))
	}
}

fn receiver_rig() -> RigNode {
	RigNode::receiver("firearm-rig", guns::FIREARM_RIG.as_str())
}

impl FirearmComponents for FirearmConcept {
	fn rig_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<RigNode> {
		Layers::from_labeled("receiver", vec![receiver_rig()])
	}

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
	fn bullpup_assembles_body_barrel_grip_on_the_receiver() {
		let gun = FirearmConcept::Bullpup;
		let rigs = gun.rig_nodes_for_level(LodSceneLevel::High).flatten();
		let bodies = gun.body_nodes_for_level(LodSceneLevel::High).flatten();
		let barrels = gun.barrel_nodes_for_level(LodSceneLevel::High).flatten();
		let grips = gun.grip_nodes_for_level(LodSceneLevel::High).flatten();
		assert_eq!(rigs[0].scene.path, guns::FIREARM_RIG.as_str());
		assert_eq!(bodies[0].slot, FirearmPartSlot::Body);
		assert_eq!(barrels[0].slot, FirearmPartSlot::Barrel);
		assert_eq!(grips[0].slot, FirearmPartSlot::Grip);
		assert_eq!(bodies[0].socket, Some(SocketRef::bone("body")));
		assert_eq!(barrels[0].socket, Some(SocketRef::bone("barrel")));
		assert_eq!(grips[0].socket, Some(SocketRef::bone("grip")));
		assert!(gun.stock_nodes_for_level(LodSceneLevel::High).is_empty());
	}

	#[test]
	fn every_concept_emits_the_shared_receiver() {
		for gun in FirearmConcept::ALL {
			let rigs = gun.rig_nodes_for_level(LodSceneLevel::High).flatten();
			assert_eq!(rigs.len(), 1);
			assert_eq!(rigs[0].scene.path, guns::FIREARM_RIG.as_str());
		}
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
			assert!(gun.stock_nodes_for_level(LodSceneLevel::High).is_empty());
		}
	}

	#[test]
	fn laznard_is_barrel_only() {
		let gun = FirearmConcept::Laznard;
		assert!(gun.body_nodes_for_level(LodSceneLevel::High).is_empty());
		assert_eq!(gun.barrel_nodes_for_level(LodSceneLevel::High).len(), 1);
	}
}
