use character_ui_menu::{
	AssetSingleSelect, CameraFocus, MenuNode, MenuTree, PreviewColor, Section, SwatchSingleSelect,
};
use crozon_character_items::{ClothingMesh, ItemColor};
use crozon_characters::{
	species::lero::{
		LeroColors, LeroConfig, LeroEyeColor, LeroHeadMesh, LeroMouthColor, LeroMouthMesh,
		LeroSkinColor, LeroSpineColor, LeroTailColor,
	},
	ConceptAnimation,
};

use crate::{
	event::{AssetValue, CharacterField, MenuEvent, SwatchValue},
	focus::{BODY_FOCUS, EYE_FOCUS, HEAD_ROOT_FOCUS, MOUTH_FOCUS},
	shared::{AnimationMenu, ClothingMenu, HairMenu},
};

#[derive(Clone, Debug, PartialEq)]
pub struct LeroHeadMenu {
	pub head: AssetSingleSelect<LeroHeadMesh>,
	pub skin: SwatchSingleSelect<LeroSkinColor>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LeroHeadFeaturesMenu {
	pub snout: AssetSingleSelect<LeroMouthMesh>,
	pub mouth_color: SwatchSingleSelect<LeroMouthColor>,
	pub eye_color: SwatchSingleSelect<LeroEyeColor>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LeroBodyMenu {
	pub tail_color: SwatchSingleSelect<LeroTailColor>,
	pub spine_color: SwatchSingleSelect<LeroSpineColor>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LeroMenu {
	pub head: Section<LeroHeadMenu>,
	pub head_features: Section<LeroHeadFeaturesMenu>,
	pub body: Section<LeroBodyMenu>,
	pub hair: Section<HairMenu>,
	pub clothing: Section<ClothingMenu>,
	pub animation: Section<AnimationMenu>,
}

impl From<&LeroConfig> for LeroMenu {
	fn from(config: &LeroConfig) -> Self {
		Self {
			head: Section::new(
				"Head",
				LeroHeadMenu {
					head: AssetSingleSelect::new(LeroHeadMesh::OrthoTee)
						.with_camera_focus(HEAD_ROOT_FOCUS),
					skin: SwatchSingleSelect::new(config.colors.skin),
				},
			),
			head_features: Section::new(
				"Head & Features",
				LeroHeadFeaturesMenu {
					snout: AssetSingleSelect::new(config.mouth).with_camera_focus(MOUTH_FOCUS),
					mouth_color: SwatchSingleSelect::new(config.colors.mouth)
						.with_camera_focus(MOUTH_FOCUS),
					eye_color: SwatchSingleSelect::new(config.colors.eyes)
						.with_camera_focus(EYE_FOCUS),
				},
			),
			body: Section::new(
				"Body & Accents",
				LeroBodyMenu {
					tail_color: SwatchSingleSelect::new(config.colors.tail),
					spine_color: SwatchSingleSelect::new(config.colors.spine),
				},
			)
			.with_camera_focus(BODY_FOCUS),
			hair: Section::new(
				"Hair",
				HairMenu::new(config.hair, config.colors.hair, HEAD_ROOT_FOCUS),
			),
			clothing: Section::new(
				"Clothing",
				ClothingMenu::new(
					config.clothing.clone(),
					config.colors.clothing_default,
					config.colors.clothing.clone(),
				),
			)
			.with_camera_focus(BODY_FOCUS),
			animation: Section::new("Animation", AnimationMenu::new(BODY_FOCUS)),
		}
	}
}

impl From<&LeroMenu> for LeroConfig {
	fn from(menu: &LeroMenu) -> Self {
		Self {
			mouth: menu.head_features.value.snout.value,
			hair: menu.hair.value.style.value,
			clothing: menu.clothing.value.layers.selected.clone(),
			colors: LeroColors {
				skin: menu.head.value.skin.value,
				eyes: menu.head_features.value.eye_color.value,
				mouth: menu.head_features.value.mouth_color.value,
				tail: menu.body.value.tail_color.value,
				spine: menu.body.value.spine_color.value,
				hair: menu.hair.value.color.value,
				clothing_default: menu.clothing.value.default_color.value,
				clothing: menu.clothing.value.item_colors.clone(),
			},
		}
	}
}

impl MenuTree<MenuEvent> for LeroHeadMenu {
	fn menu_nodes(&self) -> Vec<MenuNode<MenuEvent>> {
		vec![
			MenuNode::asset_grid("Head", &self.head, PreviewColor::of(self.skin.value), |value| {
				MenuEvent::SetAsset(CharacterField::LeroHead, AssetValue::LeroHead(value))
			}),
			MenuNode::swatch("Skin", &self.skin, |color| {
				MenuEvent::SetSwatch(CharacterField::LeroSkinColor, SwatchValue::LeroSkin(color))
			}),
		]
	}
}

impl MenuTree<MenuEvent> for LeroHeadFeaturesMenu {
	fn menu_nodes(&self) -> Vec<MenuNode<MenuEvent>> {
		vec![
			MenuNode::asset_grid(
				"Snout",
				&self.snout,
				PreviewColor::of(self.mouth_color.value),
				|value| {
					MenuEvent::SetAsset(CharacterField::LeroMouth, AssetValue::LeroMouth(value))
				},
			),
			MenuNode::swatch("Mouth Color", &self.mouth_color, |color| {
				MenuEvent::SetSwatch(
					CharacterField::LeroMouthColor,
					SwatchValue::LeroMouthColor(color),
				)
			}),
			MenuNode::swatch("Eye Color", &self.eye_color, |color| {
				MenuEvent::SetSwatch(CharacterField::LeroEyeColor, SwatchValue::LeroEye(color))
			}),
		]
	}
}

impl MenuTree<MenuEvent> for LeroBodyMenu {
	fn menu_nodes(&self) -> Vec<MenuNode<MenuEvent>> {
		vec![
			MenuNode::swatch("Tail Color", &self.tail_color, |color| {
				MenuEvent::SetSwatch(CharacterField::LeroTailColor, SwatchValue::LeroTail(color))
			}),
			MenuNode::swatch("Spine Color", &self.spine_color, |color| {
				MenuEvent::SetSwatch(CharacterField::LeroSpineColor, SwatchValue::LeroSpine(color))
			}),
		]
	}
}

impl MenuTree<MenuEvent> for LeroMenu {
	fn menu_nodes(&self) -> Vec<MenuNode<MenuEvent>> {
		vec![
			MenuNode::section(self.head.label, self.head.value.menu_nodes()),
			MenuNode::section(self.head_features.label, self.head_features.value.menu_nodes()),
			MenuNode::section(self.body.label, self.body.value.menu_nodes()),
			MenuNode::section(self.hair.label, self.hair.value.menu_nodes()),
			MenuNode::section(self.clothing.label, self.clothing.value.menu_nodes()),
			MenuNode::section(self.animation.label, self.animation.value.menu_nodes()),
		]
	}
}

impl LeroMenu {
	pub fn with_animation(mut self, animation: ConceptAnimation) -> Self {
		self.animation.value.clip.value = animation;
		self
	}

	pub fn animation(&self) -> ConceptAnimation {
		self.animation.value.clip.value
	}

	pub fn clothing_color(&self, clothing: ClothingMesh) -> ItemColor {
		self.clothing.value.color_for(clothing)
	}

	pub fn set_clothing_color(&mut self, clothing: ClothingMesh, color: ItemColor) {
		self.clothing.value.set_color(clothing, color);
	}

	pub fn camera_focus_for_field(&self, field: CharacterField) -> Option<CameraFocus> {
		match field {
			CharacterField::LeroHead => self.head.value.head.camera_focus,
			CharacterField::LeroMouth => self.head_features.value.snout.camera_focus,
			CharacterField::LeroMouthColor => self.head_features.value.mouth_color.camera_focus,
			CharacterField::LeroEyeColor => self.head_features.value.eye_color.camera_focus,
			CharacterField::Hair => self.hair.value.style.camera_focus,
			CharacterField::Clothing(_)
			| CharacterField::Animation
			| CharacterField::LeroTailColor
			| CharacterField::LeroSpineColor => Some(BODY_FOCUS),
			_ => None,
		}
	}
}

impl Default for LeroMenu {
	fn default() -> Self {
		Self::from(&LeroConfig::default_preview())
	}
}
