use character_ui_menu::{AssetSingleSelect, CameraFocus, MultiSelect, Section, SwatchSingleSelect};
use crozon_characters::{
	species::{
		braidman::{BraidmanColor, ClothingColor},
		common::{ClothingMesh, HairMesh},
		lero::{
			LeroEyeColor, LeroHeadMesh, LeroMouthMesh, LeroSkinColor, LeroSpineColor, LeroTailColor,
		},
	},
	ConceptAnimation,
};

use crate::{
	characters::braidman::AnimationMenu,
	event::CharacterField,
	focus::{BODY_FOCUS, EYE_FOCUS, HEAD_ROOT_FOCUS, MOUTH_FOCUS},
};

#[derive(Clone, Debug, PartialEq)]
pub struct LeroHeadMenu {
	pub head: AssetSingleSelect<LeroHeadMesh>,
	pub skin: SwatchSingleSelect<LeroSkinColor>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LeroHeadFeaturesMenu {
	pub snout: AssetSingleSelect<LeroMouthMesh>,
	pub eye_color: SwatchSingleSelect<LeroEyeColor>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LeroBodyMenu {
	pub tail_color: SwatchSingleSelect<LeroTailColor>,
	pub spine_color: SwatchSingleSelect<LeroSpineColor>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LeroHairMenu {
	pub style: AssetSingleSelect<HairMesh>,
	pub color: SwatchSingleSelect<BraidmanColor>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LeroClothingMenu {
	pub layers: MultiSelect<ClothingMesh>,
	pub default_color: SwatchSingleSelect<BraidmanColor>,
	pub item_colors: Vec<ClothingColor>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LeroMenu {
	pub head: Section<LeroHeadMenu>,
	pub head_features: Section<LeroHeadFeaturesMenu>,
	pub body: Section<LeroBodyMenu>,
	pub hair: Section<LeroHairMenu>,
	pub clothing: Section<LeroClothingMenu>,
	pub animation: Section<AnimationMenu>,
}

impl From<&crozon_characters::species::lero::LeroConfig> for LeroMenu {
	fn from(config: &crozon_characters::species::lero::LeroConfig) -> Self {
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
				LeroHairMenu {
					style: AssetSingleSelect::new(config.hair).with_camera_focus(HEAD_ROOT_FOCUS),
					color: SwatchSingleSelect::new(config.colors.hair),
				},
			),
			clothing: Section::new(
				"Clothing",
				LeroClothingMenu {
					layers: MultiSelect::new(config.clothing.clone()),
					default_color: SwatchSingleSelect::new(config.colors.clothing_default),
					item_colors: config.colors.clothing.clone(),
				},
			)
			.with_camera_focus(BODY_FOCUS),
			animation: Section::new(
				"Animation",
				AnimationMenu {
					clip: AssetSingleSelect::new(ConceptAnimation::Still)
						.with_camera_focus(BODY_FOCUS),
				},
			),
		}
	}
}

impl From<&LeroMenu> for crozon_characters::species::lero::LeroConfig {
	fn from(menu: &LeroMenu) -> Self {
		Self {
			mouth: menu.head_features.value.snout.value,
			hair: menu.hair.value.style.value,
			clothing: menu.clothing.value.layers.selected.clone(),
			colors: crozon_characters::species::lero::LeroColors {
				skin: menu.head.value.skin.value,
				eyes: menu.head_features.value.eye_color.value,
				tail: menu.body.value.tail_color.value,
				spine: menu.body.value.spine_color.value,
				hair: menu.hair.value.color.value,
				clothing_default: menu.clothing.value.default_color.value,
				clothing: menu.clothing.value.item_colors.clone(),
			},
		}
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

	pub fn clothing_color(&self, clothing: ClothingMesh) -> BraidmanColor {
		self.clothing
			.value
			.item_colors
			.iter()
			.find(|choice| choice.clothing == clothing)
			.map(|choice| choice.color)
			.unwrap_or(self.clothing.value.default_color.value)
	}

	pub fn set_clothing_color(&mut self, clothing: ClothingMesh, color: BraidmanColor) {
		if let Some(choice) = self
			.clothing
			.value
			.item_colors
			.iter_mut()
			.find(|choice| choice.clothing == clothing)
		{
			choice.color = color;
		} else {
			self.clothing.value.item_colors.push(ClothingColor { clothing, color });
		}
	}

	pub fn camera_focus_for_field(&self, field: CharacterField) -> Option<CameraFocus> {
		match field {
			CharacterField::LeroHead => self.head.value.head.camera_focus,
			CharacterField::LeroMouth => self.head_features.value.snout.camera_focus,
			CharacterField::LeroEyeColor => self.head_features.value.eye_color.camera_focus,
			CharacterField::Hair => self.hair.value.style.camera_focus,
			CharacterField::Clothing(_) | CharacterField::Animation | CharacterField::LeroTailColor
			| CharacterField::LeroSpineColor => Some(BODY_FOCUS),
			_ => None,
		}
	}
}

impl Default for LeroMenu {
	fn default() -> Self {
		Self::from(&crozon_characters::species::lero::LeroConfig::default_preview())
	}
}
