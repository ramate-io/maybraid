use crozon_characters::species::{
	braidman::BraidmanConfig, brodler::BrodlerConfig, common::ClothingMesh,
};

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
fn clothing_toggle_and_color() -> anyhow::Result<()> {
	use crozon_characters::species::braidman::BraidmanColor;

	use crate::{CharacterField, MenuEvent, SwatchValue};

	let mut menu = CharacterMenu::default();
	let coat = ClothingMesh::FittedCoat;
	assert!(menu.apply(MenuEvent::ToggleClothing(coat)));
	assert!(menu.braidman.clothing.value.layers.contains(coat));
	assert!(menu.apply(MenuEvent::SetSwatch(
		CharacterField::Clothing(coat),
		SwatchValue::Braidman(BraidmanColor::Red),
	)));
	assert_eq!(menu.braidman.clothing_color(coat), BraidmanColor::Red);
	Ok(())
}

#[test]
fn body_color_syncs_skin() -> anyhow::Result<()> {
	use crozon_characters::species::braidman::BraidmanColor;

	use crate::{CharacterField, MenuEvent, SwatchValue};

	let mut menu = CharacterMenu::default();
	assert!(menu.apply(MenuEvent::SetSwatch(
		CharacterField::BodyColor,
		SwatchValue::Braidman(BraidmanColor::Warm),
	)));
	let config = menu.braidman_config();
	assert_eq!(config.colors.body, BraidmanColor::Warm);
	assert_eq!(config.colors.head, BraidmanColor::Warm);
	assert_eq!(config.colors.nose, BraidmanColor::Warm);
	Ok(())
}
