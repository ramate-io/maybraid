use character_ui_menu::{AssetSingleSelect, CameraFocus, MultiSelect, Section, SwatchSingleSelect};
use crozon_characters::{
	species::{
		braidman::{BraidmanColor, ClothingColor},
		brodler::{
			assets::HornMesh, BrodlerColors, BrodlerConfig, BrodlerEyeColor, BrodlerHeadMesh,
			BrodlerHornColor, BrodlerSkinColor,
		},
		common::{ClothingMesh, EarMesh, EyeMesh, HairMesh, MouthMesh, NoseMesh},
	},
	ConceptAnimation,
};

use crate::{
	characters::braidman::AnimationMenu,
	event::CharacterField,
	focus::{
		BODY_FOCUS, CROWN_FOCUS, EAR_FOCUS, EYE_FOCUS, HEAD_ROOT_FOCUS, MOUTH_FOCUS, NOSE_FOCUS,
	},
};

#[derive(Clone, Debug, PartialEq)]
pub struct BrodlerHeadMenu {
	pub head: AssetSingleSelect<BrodlerHeadMesh>,
	pub horns: AssetSingleSelect<HornMesh>,
	pub skin: SwatchSingleSelect<BrodlerSkinColor>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BrodlerHeadFeaturesMenu {
	pub eye: AssetSingleSelect<EyeMesh>,
	pub nose: AssetSingleSelect<NoseMesh>,
	pub mouth: AssetSingleSelect<MouthMesh>,
	pub ear: AssetSingleSelect<EarMesh>,
	pub eye_color: SwatchSingleSelect<BrodlerEyeColor>,
	pub horn_color: SwatchSingleSelect<BrodlerHornColor>,
	pub mouth_color: SwatchSingleSelect<BraidmanColor>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BrodlerHairMenu {
	pub style: AssetSingleSelect<HairMesh>,
	pub color: SwatchSingleSelect<BraidmanColor>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BrodlerClothingMenu {
	pub layers: MultiSelect<ClothingMesh>,
	pub default_color: SwatchSingleSelect<BraidmanColor>,
	pub item_colors: Vec<ClothingColor>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BrodlerMenu {
	pub head: Section<BrodlerHeadMenu>,
	pub head_features: Section<BrodlerHeadFeaturesMenu>,
	pub hair: Section<BrodlerHairMenu>,
	pub clothing: Section<BrodlerClothingMenu>,
	pub animation: Section<AnimationMenu>,
}

impl From<&BrodlerConfig> for BrodlerMenu {
	fn from(config: &BrodlerConfig) -> Self {
		Self {
			head: Section::new(
				"Head",
				BrodlerHeadMenu {
					head: AssetSingleSelect::new(config.head).with_camera_focus(HEAD_ROOT_FOCUS),
					horns: AssetSingleSelect::new(config.horns).with_camera_focus(CROWN_FOCUS),
					skin: SwatchSingleSelect::new(config.colors.skin),
				},
			),
			head_features: Section::new(
				"Head & Features",
				BrodlerHeadFeaturesMenu {
					eye: AssetSingleSelect::new(config.eye).with_camera_focus(EYE_FOCUS),
					nose: AssetSingleSelect::new(config.nose).with_camera_focus(NOSE_FOCUS),
					mouth: AssetSingleSelect::new(config.mouth).with_camera_focus(MOUTH_FOCUS),
					ear: AssetSingleSelect::new(config.ear).with_camera_focus(EAR_FOCUS),
					eye_color: SwatchSingleSelect::new(config.colors.eyes),
					horn_color: SwatchSingleSelect::new(config.colors.horns),
					mouth_color: SwatchSingleSelect::new(config.colors.mouth),
				},
			),
			hair: Section::new(
				"Hair",
				BrodlerHairMenu {
					style: AssetSingleSelect::new(config.hair).with_camera_focus(CROWN_FOCUS),
					color: SwatchSingleSelect::new(config.colors.hair),
				},
			),
			clothing: Section::new(
				"Clothing",
				BrodlerClothingMenu {
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

impl From<&BrodlerMenu> for BrodlerConfig {
	fn from(menu: &BrodlerMenu) -> Self {
		Self {
			head: menu.head.value.head.value,
			horns: menu.head.value.horns.value,
			eye: menu.head_features.value.eye.value,
			nose: menu.head_features.value.nose.value,
			mouth: menu.head_features.value.mouth.value,
			ear: menu.head_features.value.ear.value,
			hair: menu.hair.value.style.value,
			clothing: menu.clothing.value.layers.selected.clone(),
			colors: BrodlerColors {
				skin: menu.head.value.skin.value,
				eyes: menu.head_features.value.eye_color.value,
				horns: menu.head_features.value.horn_color.value,
				mouth: menu.head_features.value.mouth_color.value,
				hair: menu.hair.value.color.value,
				clothing_default: menu.clothing.value.default_color.value,
				clothing: menu.clothing.value.item_colors.clone(),
			},
		}
	}
}

impl BrodlerMenu {
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
			CharacterField::BrodlerHead => self.head.value.head.camera_focus,
			CharacterField::Horns => self.head.value.horns.camera_focus,
			CharacterField::Eye => self.head_features.value.eye.camera_focus,
			CharacterField::Nose => self.head_features.value.nose.camera_focus,
			CharacterField::Mouth => self.head_features.value.mouth.camera_focus,
			CharacterField::Ear => self.head_features.value.ear.camera_focus,
			CharacterField::Hair => self.hair.value.style.camera_focus,
			CharacterField::Clothing(_) | CharacterField::Animation => Some(BODY_FOCUS),
			_ => None,
		}
	}
}

impl Default for BrodlerMenu {
	fn default() -> Self {
		Self::from(&BrodlerConfig::default_preview())
	}
}
