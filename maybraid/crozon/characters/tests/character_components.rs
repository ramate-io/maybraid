//! CharacterComponents / Clothed composition tests.

use crozon_character_items::ClothingMesh;
use crozon_characters::{
	species::{
		braidman::{bsn::Braidman, BraidmanConfig},
		brodler::{bsn::Brodler, BrodlerConfig},
		common::nodes as humanoid,
	},
	CharacterComponents, CharacterRecipe, Clothed, Layer,
};
use lod::gen::LodSceneLevel;
use scene_ref::MirrorAxis;

fn assert_right_features_reflected(parts: &[crozon_characters::PartNode]) {
	let right_ear = parts
		.iter()
		.find(|p| p.slot == crozon_characters::CharacterPartSlot::EarRight)
		.expect("right ear");
	assert_eq!(right_ear.scene.mirror, Some(MirrorAxis::X));
	assert!(right_ear.scene.reflect_instance);
	let right_eye = parts
		.iter()
		.find(|p| p.slot == crozon_characters::CharacterPartSlot::EyeRight)
		.expect("right eye");
	assert_eq!(right_eye.scene.mirror, Some(MirrorAxis::X));
	assert!(right_eye.scene.reflect_instance);
}

fn assert_ear_socket_locals(parts: &[crozon_characters::PartNode]) {
	let local = |slot| {
		parts
			.iter()
			.find(|p| p.slot == slot)
			.and_then(|p| p.socket)
			.map(|s| s.local)
			.expect("socket")
	};
	let left_ear = local(crozon_characters::CharacterPartSlot::EarLeft);
	let right_ear = local(crozon_characters::CharacterPartSlot::EarRight);
	assert_eq!(left_ear, humanoid::ear_left_socket_local());
	assert_eq!(right_ear, humanoid::ear_right_socket_local());
	assert_ne!(left_ear, right_ear);
	assert!(right_ear.translation.x < 0.0);
	assert_eq!(
		local(crozon_characters::CharacterPartSlot::EyeLeft),
		local(crozon_characters::CharacterPartSlot::EyeRight)
	);
}

#[test]
fn braidman_emits_rigs_and_parts_at_every_band() {
	let braidman = Braidman::from_config(&BraidmanConfig::default_preview());
	let high_rigs = braidman.rig_nodes_for_level(LodSceneLevel::High);
	let low_rigs = braidman.rig_nodes_for_level(LodSceneLevel::Low);
	assert_eq!(high_rigs.len(), 2);
	assert_eq!(high_rigs.len(), low_rigs.len());

	let high_parts = braidman.part_nodes_for_level(LodSceneLevel::High);
	let ultra = braidman.part_nodes_for_level(LodSceneLevel::UltraLow);
	assert_eq!(high_parts.len(), ultra.len());
	assert!(high_parts.len() >= 8);
}

#[test]
fn clothed_braidman_adds_clothing_layer() {
	let mut config = BraidmanConfig::default_preview();
	config.clothing.push(ClothingMesh::Tunic);
	let inner = Braidman::from_config(&config);
	let clothed = config.clothed();
	let inner_len = inner.part_nodes_for_level(LodSceneLevel::High).len();
	let clothed_parts = clothed.part_nodes_for_level(LodSceneLevel::High);
	assert_eq!(clothed_parts.len(), inner_len + 1);
	assert!(clothed_parts.labeled.contains_key(&Layer::new("clothing")));
	assert_eq!(clothed.rig_nodes_for_level(LodSceneLevel::High).len(), 2);
}

#[test]
fn right_features_use_scene_ref_instance_reflect() {
	let braidman = Braidman::from_config(&BraidmanConfig::default_preview());
	assert_right_features_reflected(&braidman.part_nodes_for_level(LodSceneLevel::High).flatten());
}

#[test]
fn right_ear_keeps_right_socket_local() {
	let braidman = Braidman::from_config(&BraidmanConfig::default_preview());
	assert_ear_socket_locals(&braidman.part_nodes_for_level(LodSceneLevel::High).flatten());
}

#[test]
fn clothed_from_config_matches_helper() {
	let config = BraidmanConfig::default_preview();
	let a = config.clothed();
	let b = Clothed::new(Braidman::from_config(&config), Vec::new());
	assert_eq!(
		a.part_nodes_for_level(LodSceneLevel::High).len(),
		b.part_nodes_for_level(LodSceneLevel::High).len()
	);
}

#[test]
fn brodler_emits_rigs_and_parts_at_every_band() {
	let brodler = Brodler::from_config(&BrodlerConfig::default_preview());
	let high_rigs = brodler.rig_nodes_for_level(LodSceneLevel::High);
	assert_eq!(high_rigs.len(), 2);
	assert_eq!(high_rigs.len(), brodler.rig_nodes_for_level(LodSceneLevel::Low).len());

	let high_parts = brodler.part_nodes_for_level(LodSceneLevel::High);
	assert_eq!(high_parts.len(), brodler.part_nodes_for_level(LodSceneLevel::UltraLow).len());
	assert!(high_parts.len() >= 9);
}

#[test]
fn clothed_brodler_adds_clothing_layer() {
	let mut config = BrodlerConfig::default_preview();
	config.clothing.push(ClothingMesh::Tunic);
	let inner = Brodler::from_config(&config);
	let clothed = CharacterRecipe::clothed(&config);
	let inner_len = inner.part_nodes_for_level(LodSceneLevel::High).len();
	let clothed_parts = clothed.part_nodes_for_level(LodSceneLevel::High);
	assert_eq!(clothed_parts.len(), inner_len + 1);
	assert!(clothed_parts.labeled.contains_key(&Layer::new("clothing")));
}

#[test]
fn brodler_right_features_match_shared_humanoid_helpers() {
	let brodler = Brodler::from_config(&BrodlerConfig::default_preview());
	let parts = brodler.part_nodes_for_level(LodSceneLevel::High).flatten();
	assert_right_features_reflected(&parts);
	assert_ear_socket_locals(&parts);
}
