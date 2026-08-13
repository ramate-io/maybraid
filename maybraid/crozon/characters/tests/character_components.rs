//! CharacterComponents / Clothed<Braidman> composition tests.

use crozon_character_items::ClothingMesh;
use crozon_characters::{
	species::braidman::{bsn::Braidman, BraidmanConfig},
	CharacterComponents, Clothed, Layer,
};
use lod::gen::LodSceneLevel;
use scene_ref::MirrorAxis;

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
fn right_features_use_scene_ref_mirror() {
	let braidman = Braidman::from_config(&BraidmanConfig::default_preview());
	let parts = braidman.part_nodes_for_level(LodSceneLevel::High).flatten();
	let right_ear = parts
		.iter()
		.find(|p| p.slot == crozon_characters::CharacterPartSlot::EarRight)
		.expect("right ear");
	assert_eq!(right_ear.scene.mirror, Some(MirrorAxis::X));
	let right_eye = parts
		.iter()
		.find(|p| p.slot == crozon_characters::CharacterPartSlot::EyeRight)
		.expect("right eye");
	assert_eq!(right_eye.scene.mirror, Some(MirrorAxis::X));
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
