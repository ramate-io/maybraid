use character_ui_menu::{MenuComponent, MenuNode};
use crozon_character_items::ClothingMesh;
use crozon_characters::species::{braidman::BraidmanConfig, brodler::BrodlerConfig};

use crate::{
	character::CharacterMenu,
	characters::{braidman::BraidmanMenu, brodler::BrodlerMenu},
};

#[test]
fn braidman_config_round_trip() -> anyhow::Result<()> {
	let config = BraidmanConfig::default_preview();
	let menu = BraidmanMenu::from(&config);
	let restored = BraidmanConfig::from(&menu);
	assert_eq!(config.gender, restored.gender);
	assert_eq!(config.body, restored.body);
	assert_eq!(config.sliders.shoulder_width, restored.sliders.shoulder_width);
	Ok(())
}

#[test]
fn brenal_config_round_trip() -> anyhow::Result<()> {
	let config = crozon_characters::species::brenal::BrenalConfig::default_preview();
	let menu = crate::characters::brenal::BrenalMenu::from(&config);
	let restored = crozon_characters::species::brenal::BrenalConfig::from(&menu);
	assert_eq!(config.gender, restored.gender);
	assert_eq!(config.horns, restored.horns);
	assert_eq!(config.sliders.shoulder_width, restored.sliders.shoulder_width);
	Ok(())
}

#[test]
fn caole_config_round_trip() -> anyhow::Result<()> {
	let config = crozon_characters::species::caole::CaoleConfig::default_preview();
	let menu = crate::characters::caole::CaoleMenu::from(&config);
	let restored = crozon_characters::species::caole::CaoleConfig::from(&menu);
	assert_eq!(config.gender, restored.gender);
	assert_eq!(config.body, restored.body);
	assert_eq!(config.mouth, restored.mouth);
	assert_eq!(config.sliders.shoulder_width, restored.sliders.shoulder_width);
	Ok(())
}

#[test]
fn epiphant_config_round_trip() -> anyhow::Result<()> {
	let config = crozon_characters::species::epiphant::EpiphantConfig::default_preview();
	let menu = crate::characters::epiphant::EpiphantMenu::from(&config);
	let restored = crozon_characters::species::epiphant::EpiphantConfig::from(&menu);
	assert_eq!(config.gender, restored.gender);
	assert_eq!(config.body, restored.body);
	assert_eq!(config.nose, restored.nose);
	assert_eq!(config.eye, restored.eye);
	assert_eq!(config.sliders.shoulder_width, restored.sliders.shoulder_width);
	Ok(())
}

#[test]
fn hars_config_round_trip() -> anyhow::Result<()> {
	let config = crozon_characters::species::hars::HarsConfig::default_preview();
	let menu = crate::characters::hars::HarsMenu::from(&config);
	let restored = crozon_characters::species::hars::HarsConfig::from(&menu);
	assert_eq!(config.gender, restored.gender);
	assert_eq!(config.mouth, restored.mouth);
	assert_eq!(config.sliders.shoulder_width, restored.sliders.shoulder_width);
	Ok(())
}

#[test]
fn ylter_config_round_trip() -> anyhow::Result<()> {
	let config = crozon_characters::species::ylter::YilterConfig::default_preview();
	let menu = crate::characters::ylter::YilterMenu::from(&config);
	let restored = crozon_characters::species::ylter::YilterConfig::from(&menu);
	assert_eq!(config.gender, restored.gender);
	assert_eq!(config.mouth, restored.mouth);
	assert_eq!(config.sliders.shoulder_width, restored.sliders.shoulder_width);
	Ok(())
}

#[test]
fn sonyak_config_round_trip() -> anyhow::Result<()> {
	let config = crozon_characters::species::sonyak::SonyakConfig::default_preview();
	let menu = crate::characters::sonyak::SonyakMenu::from(&config);
	let restored = crozon_characters::species::sonyak::SonyakConfig::from(&menu);
	assert_eq!(config.gender, restored.gender);
	assert_eq!(config.mouth, restored.mouth);
	assert_eq!(config.colors.hair, restored.colors.hair);
	assert_eq!(config.sliders.shoulder_width, restored.sliders.shoulder_width);
	Ok(())
}

#[test]
fn claber_config_round_trip() -> anyhow::Result<()> {
	let config = crozon_characters::species::claber::ClaberConfig::default_preview();
	let menu = crate::characters::claber::ClaberMenu::from(&config);
	let restored = crozon_characters::species::claber::ClaberConfig::from(&menu);
	assert_eq!(config.gender, restored.gender);
	assert_eq!(config.horns, restored.horns);
	assert_eq!(config.sliders.arm_length, restored.sliders.arm_length);
	assert_eq!(config.colors.body, restored.colors.body);
	Ok(())
}

#[test]
fn croconot_config_round_trip() -> anyhow::Result<()> {
	let config = crozon_characters::species::croconot::CroconotConfig::default_preview();
	let menu = crate::characters::croconot::CroconotMenu::from(&config);
	let restored = crozon_characters::species::croconot::CroconotConfig::from(&menu);
	assert_eq!(config.gender, restored.gender);
	assert_eq!(config.horns, restored.horns);
	assert_eq!(config.sliders.arm_length, restored.sliders.arm_length);
	assert_eq!(config.colors.body, restored.colors.body);
	Ok(())
}

#[test]
fn brodler_config_round_trip() -> anyhow::Result<()> {
	let config = BrodlerConfig::default_preview();
	let menu = BrodlerMenu::from(&config);
	let restored = BrodlerConfig::from(&menu);
	assert_eq!(config.head, restored.head);
	assert_eq!(config.horns, restored.horns);
	assert_eq!(config.colors.skin, restored.colors.skin);
	Ok(())
}

#[test]
fn dui_config_round_trip() -> anyhow::Result<()> {
	let config = crozon_characters::species::dui::DuiConfig::default_preview();
	let menu = crate::characters::dui::DuiMenu::from(&config);
	let restored = crozon_characters::species::dui::DuiConfig::from(&menu);
	assert_eq!(config.nose, restored.nose);
	assert_eq!(config.colors.skin, restored.colors.skin);
	assert_eq!(config.colors.mouth, restored.colors.mouth);
	assert_eq!(config.colors.eyes, restored.colors.eyes);
	assert_eq!(config.colors.nose_color, restored.colors.nose_color);
	Ok(())
}

#[test]
fn lidder_config_round_trip() -> anyhow::Result<()> {
	let config = crozon_characters::species::lidder::LidderConfig::default_preview();
	let menu = crate::characters::lidder::LidderMenu::from(&config);
	let restored = crozon_characters::species::lidder::LidderConfig::from(&menu);
	assert_eq!(config.beak, restored.beak);
	assert_eq!(config.eye, restored.eye);
	assert_eq!(config.hair, restored.hair);
	assert_eq!(config.colors.plumage, restored.colors.plumage);
	assert_eq!(config.colors.eyes, restored.colors.eyes);
	assert_eq!(config.colors.beak, restored.colors.beak);
	Ok(())
}

#[test]
fn chupri_config_round_trip() -> anyhow::Result<()> {
	let config = crozon_characters::species::chupri::ChupriConfig::default_preview();
	let menu = crate::characters::chupri::ChupriMenu::from(&config);
	let restored = crozon_characters::species::chupri::ChupriConfig::from(&menu);
	assert_eq!(config.beak, restored.beak);
	assert_eq!(config.eye, restored.eye);
	assert_eq!(config.hair, restored.hair);
	assert_eq!(config.colors.plumage, restored.colors.plumage);
	assert_eq!(config.colors.eyes, restored.colors.eyes);
	assert_eq!(config.colors.beak, restored.colors.beak);
	Ok(())
}

#[test]
fn brokker_config_round_trip() -> anyhow::Result<()> {
	let config = crozon_characters::species::brokker::BrokkerConfig::default_preview();
	let menu = crate::characters::brokker::BrokkerMenu::from(&config);
	let restored = crozon_characters::species::brokker::BrokkerConfig::from(&menu);
	assert_eq!(config.eye, restored.eye);
	assert_eq!(config.hair, restored.hair);
	assert_eq!(config.colors.plumage, restored.colors.plumage);
	assert_eq!(config.colors.eyes, restored.colors.eyes);
	assert_eq!(config.colors.snout, restored.colors.snout);
	Ok(())
}

#[test]
fn tipple_config_round_trip() -> anyhow::Result<()> {
	let config = crozon_characters::species::tipple::TippleConfig::default_preview();
	let menu = crate::characters::tipple::TippleMenu::from(&config);
	let restored = crozon_characters::species::tipple::TippleConfig::from(&menu);
	assert_eq!(config.beak, restored.beak);
	assert_eq!(config.eye, restored.eye);
	assert_eq!(config.hair, restored.hair);
	assert_eq!(config.colors.plumage, restored.colors.plumage);
	assert_eq!(config.colors.eyes, restored.colors.eyes);
	assert_eq!(config.colors.beak, restored.colors.beak);
	Ok(())
}

#[test]
fn topple_config_round_trip() -> anyhow::Result<()> {
	let config = crozon_characters::species::topple::ToppleConfig::default_preview();
	let menu = crate::characters::topple::ToppleMenu::from(&config);
	let restored = crozon_characters::species::topple::ToppleConfig::from(&menu);
	assert_eq!(config.beak, restored.beak);
	assert_eq!(config.eye, restored.eye);
	assert_eq!(config.hair, restored.hair);
	assert_eq!(config.colors.plumage, restored.colors.plumage);
	assert_eq!(config.colors.eyes, restored.colors.eyes);
	assert_eq!(config.colors.beak, restored.colors.beak);
	Ok(())
}

#[test]
fn kispar_config_round_trip() -> anyhow::Result<()> {
	let config = crozon_characters::species::kispar::KisparConfig::default_preview();
	let menu = crate::characters::kispar::KisparMenu::from(&config);
	let restored = crozon_characters::species::kispar::KisparConfig::from(&menu);
	assert_eq!(config.beak, restored.beak);
	assert_eq!(config.eye, restored.eye);
	assert_eq!(config.hair, restored.hair);
	assert_eq!(config.colors.plumage, restored.colors.plumage);
	assert_eq!(config.colors.eyes, restored.colors.eyes);
	assert_eq!(config.colors.beak, restored.colors.beak);
	Ok(())
}


#[test]
fn tapp_config_round_trip() -> anyhow::Result<()> {
	let config = crozon_characters::species::tapp::TappConfig::default_preview();
	let menu = crate::characters::tapp::TappMenu::from(&config);
	let restored = crozon_characters::species::tapp::TappConfig::from(&menu);
	assert_eq!(config.beak, restored.beak);
	assert_eq!(config.eye, restored.eye);
	assert_eq!(config.hair, restored.hair);
	assert_eq!(config.colors.plumage, restored.colors.plumage);
	assert_eq!(config.colors.eyes, restored.colors.eyes);
	assert_eq!(config.colors.beak, restored.colors.beak);
	Ok(())
}

#[test]
fn kaller_config_round_trip() -> anyhow::Result<()> {
	let config = crozon_characters::species::kaller::KallerConfig::default_preview();
	let menu = crate::characters::kaller::KallerMenu::from(&config);
	let restored = crozon_characters::species::kaller::KallerConfig::from(&menu);
	assert_eq!(config.eye, restored.eye);
	assert_eq!(config.hair, restored.hair);
	assert_eq!(config.colors.plumage, restored.colors.plumage);
	assert_eq!(config.colors.eyes, restored.colors.eyes);
	assert_eq!(config.colors.snout, restored.colors.snout);
	assert_eq!(config.colors.crown, restored.colors.crown);
	Ok(())
}

#[test]
fn kappler_config_round_trip() -> anyhow::Result<()> {
	let config = crozon_characters::species::kappler::KapplerConfig::default_preview();
	let menu = crate::characters::kappler::KapplerMenu::from(&config);
	let restored = crozon_characters::species::kappler::KapplerConfig::from(&menu);
	assert_eq!(config.beak, restored.beak);
	assert_eq!(config.eye, restored.eye);
	assert_eq!(config.hair, restored.hair);
	assert_eq!(config.colors.plumage, restored.colors.plumage);
	assert_eq!(config.colors.eyes, restored.colors.eyes);
	assert_eq!(config.colors.beak, restored.colors.beak);
	Ok(())
}

#[test]
fn mygr_config_round_trip() -> anyhow::Result<()> {
	let config = crozon_characters::species::mygr::MygrConfig::default_preview();
	let menu = crate::characters::mygr::MygrMenu::from(&config);
	let restored = crozon_characters::species::mygr::MygrConfig::from(&menu);
	assert_eq!(config.eye, restored.eye);
	assert_eq!(config.colors.skin, restored.colors.skin);
	assert_eq!(config.colors.eyes, restored.colors.eyes);
	Ok(())
}

#[test]
fn wumbus_config_round_trip() -> anyhow::Result<()> {
	let config = crozon_characters::species::wumbus::WumbusConfig::default_preview();
	let menu = crate::characters::wumbus::WumbusMenu::from(&config);
	let restored = crozon_characters::species::wumbus::WumbusConfig::from(&menu);
	assert_eq!(config.horns, restored.horns);
	assert_eq!(config.colors.skin, restored.colors.skin);
	assert_eq!(config.colors.eyes, restored.colors.eyes);
	assert_eq!(config.colors.ears, restored.colors.ears);
	assert_eq!(config.colors.mouth, restored.colors.mouth);
	assert_eq!(config.colors.horns, restored.colors.horns);
	assert_eq!(config.colors.spine, restored.colors.spine);
	Ok(())
}

#[test]
fn lero_config_round_trip() -> anyhow::Result<()> {
	let config = crozon_characters::species::lero::LeroConfig::default_preview();
	let menu = crate::characters::lero::LeroMenu::from(&config);
	let restored = crozon_characters::species::lero::LeroConfig::from(&menu);
	assert_eq!(config.mouth, restored.mouth);
	assert_eq!(config.colors.skin, restored.colors.skin);
	assert_eq!(config.colors.mouth, restored.colors.mouth);
	assert_eq!(config.colors.tail, restored.colors.tail);
	assert_eq!(config.colors.spine, restored.colors.spine);
	Ok(())
}

#[test]
fn spibmom_config_round_trip() -> anyhow::Result<()> {
	let config = crozon_characters::species::spibmom::SpibmomConfig::default_preview();
	let menu = crate::characters::spibmom::SpibmomMenu::from(&config);
	let restored = crozon_characters::species::spibmom::SpibmomConfig::from(&menu);
	assert_eq!(config.eye, restored.eye);
	assert_eq!(config.colors.skin, restored.colors.skin);
	assert_eq!(config.colors.crown, restored.colors.crown);
	assert_eq!(config.colors.spine, restored.colors.spine);
	assert_eq!(config.colors.mouth, restored.colors.mouth);
	Ok(())
}

#[test]
fn clothing_toggle_and_color() -> anyhow::Result<()> {
	use crozon_character_items::ItemColor;

	use crate::{CharacterField, MenuEvent, SwatchValue};

	let mut menu = CharacterMenu::default();
	let coat = ClothingMesh::FittedCoat;
	assert!(menu.apply(MenuEvent::ToggleClothing(coat)));
	assert!(menu.braidman.clothing.value.layers.contains(coat));
	assert!(menu.apply(MenuEvent::SetSwatch(
		CharacterField::Clothing(coat),
		SwatchValue::Item(ItemColor::Red),
	)));
	assert_eq!(menu.braidman.clothing_color(coat), ItemColor::Red);
	Ok(())
}

#[test]
fn character_menu_lowers_to_species_select_tree() -> anyhow::Result<()> {
	let menu = CharacterMenu::default();
	let nodes = menu.menu_nodes();
	assert_eq!(nodes.len(), 1);
	let MenuNode::SectionSelect { label, choices, children } = &nodes[0] else {
		anyhow::bail!("expected a species SectionSelect at the root");
	};
	assert_eq!(*label, "Species");
	assert_eq!(choices.len(), 24);
	assert!(choices[0].selected, "default species should be braidman");
	// Braidman: presets, body, head & features, hair, clothing, animation.
	assert_eq!(children.len(), 6);
	assert!(children.iter().all(|child| matches!(child, MenuNode::Section { .. })));
	Ok(())
}

#[test]
fn clothing_swatches_only_lower_for_worn_layers() -> anyhow::Result<()> {
	use crate::MenuEvent;

	let mut menu = CharacterMenu::default();
	let coat = ClothingMesh::FittedCoat;
	assert!(menu.apply(MenuEvent::ToggleClothing(coat)));
	let nodes = menu.braidman.clothing.value.menu_nodes();
	let MenuNode::ItemMultiSelect { rows, .. } = &nodes[0] else {
		anyhow::bail!("expected clothing to lower to an ItemMultiSelect");
	};
	for row in rows {
		assert_eq!(row.asset.selected, !row.colors.is_empty());
	}
	Ok(())
}

#[test]
fn body_color_syncs_skin() -> anyhow::Result<()> {
	use crozon_character_items::ItemColor;

	use crate::{CharacterField, MenuEvent, SwatchValue};

	let mut menu = CharacterMenu::default();
	assert!(menu.apply(MenuEvent::SetSwatch(
		CharacterField::BodyColor,
		SwatchValue::Item(ItemColor::Warm),
	)));
	let config = menu.braidman_config();
	assert_eq!(config.colors.body, ItemColor::Warm);
	assert_eq!(config.colors.head, ItemColor::Warm);
	assert_eq!(config.colors.nose, ItemColor::Warm);
	Ok(())
}
