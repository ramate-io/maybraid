use character_ui_menu::{
	AssetSingleSelect, CameraFocus, MultiSelect, Section, SingleSelect, SwatchSingleSelect,
};
use crozon_characters::{
	species::{
		braidman::{BraidmanColor, ClothingColor},
		common::{ClothingMesh, HairMesh},
		dui::{DuiEyeMesh, DuiHeadMesh, DuiMouthMesh, DuiMouthColor, DuiNoseMesh, DuiSkinColor},
	},
	ConceptAnimation,
};

use crate::{
	characters::braidman::AnimationMenu,
	event::CharacterField,
	focus::{BODY_FOCUS, EYE_FOCUS, HEAD_ROOT_FOCUS, MOUTH_FOCUS, NOSE_FOCUS},
};

#[derive(Clone, Debug, PartialEq)]
pub struct DuiHeadMenu {
	pub head: AssetSingleSelect<DuiHeadMesh>,
	pub skin: SwatchSingleSelect<DuiSkinColor>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DuiHeadFeaturesMenu {
	pub eye: AssetSingleSelect<DuiEyeMesh>,
	pub nose: SingleSelect<DuiNoseMesh>,
	pub mouth: AssetSingleSelect<DuiMouthMesh>,
	pub mouth_color: SwatchSingleSelect<DuiMouthColor>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DuiHairMenu {
	pub style: AssetSingleSelect<HairMesh>,
	pub color: SwatchSingleSelect<BraidmanColor>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DuiClothingMenu {
	pub layers: MultiSelect<ClothingMesh>,
	pub default_color: SwatchSingleSelect<BraidmanColor>,
	pub item_colors: Vec<ClothingColor>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DuiMenu {
	pub head: Section<DuiHeadMenu>,
	pub head_features: Section<DuiHeadFeaturesMenu>,
	pub hair: Section<DuiHairMenu>,
	pub clothing: Section<DuiClothingMenu>,
	pub animation: Section<AnimationMenu>,
}

impl From<&crozon_characters::species::dui::DuiConfig> for DuiMenu {
	fn from(config: &crozon_characters::species::dui::DuiConfig) -> Self {
		Self {
			head: Section::new(
				"Head",
				DuiHeadMenu {
					head: AssetSingleSelect::new(DuiHeadMesh::BarredBowl)
						.with_camera_focus(HEAD_ROOT_FOCUS),
					skin: SwatchSingleSelect::new(config.colors.skin),
				},
			),
			head_features: Section::new(
				"Head & Features",
				DuiHeadFeaturesMenu {
					eye: AssetSingleSelect::new(DuiEyeMesh::Thorn)
						.with_camera_focus(EYE_FOCUS),
					nose: SingleSelect::new(config.nose),
					mouth: AssetSingleSelect::new(DuiMouthMesh::SmallCommon)
						.with_camera_focus(MOUTH_FOCUS),
					mouth_color: SwatchSingleSelect::new(config.colors.mouth),
				},
			),
			hair: Section::new(
				"Hair",
				DuiHairMenu {
					style: AssetSingleSelect::new(config.hair).with_camera_focus(HEAD_ROOT_FOCUS),
					color: SwatchSingleSelect::new(config.colors.hair),
				},
			),
			clothing: Section::new(
				"Clothing",
				DuiClothingMenu {
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

impl From<&DuiMenu> for crozon_characters::species::dui::DuiConfig {
	fn from(menu: &DuiMenu) -> Self {
		Self {
			nose: menu.head_features.value.nose.value,
			hair: menu.hair.value.style.value,
			clothing: menu.clothing.value.layers.selected.clone(),
			colors: crozon_characters::species::dui::DuiColors {
				skin: menu.head.value.skin.value,
				eyes: crozon_characters::species::dui::DuiEyeColor::Black,
				nose_color: crozon_characters::species::dui::DuiNoseColor::Black,
				mouth: menu.head_features.value.mouth_color.value,
				hair: menu.hair.value.color.value,
				clothing_default: menu.clothing.value.default_color.value,
				clothing: menu.clothing.value.item_colors.clone(),
			},
		}
	}
}

impl DuiMenu {
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
			CharacterField::DuiHead => self.head.value.head.camera_focus,
			CharacterField::DuiEye => self.head_features.value.eye.camera_focus,
			CharacterField::DuiNose => {
				if self.head_features.value.nose.value == DuiNoseMesh::None {
					None
				} else {
					Some(NOSE_FOCUS)
				}
			}
			CharacterField::DuiMouth => self.head_features.value.mouth.camera_focus,
			CharacterField::Hair => self.hair.value.style.camera_focus,
			CharacterField::Clothing(_) | CharacterField::Animation => Some(BODY_FOCUS),
			_ => None,
		}
	}
}

impl Default for DuiMenu {
	fn default() -> Self {
		Self::from(&crozon_characters::species::dui::DuiConfig::default_preview())
	}
}
