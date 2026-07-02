use character_ui_menu::{AssetSingleSelect, CameraFocus, MultiSelect, Section, SwatchSingleSelect};
use crozon_characters::{
	species::{
		braidman::{BraidmanColor, ClothingColor},
		common::{ClothingMesh, EyeMesh, HairMesh},
		mygr::{MygrEyeColor, MygrHeadMesh, MygrMouthMesh, MygrSkinColor},
	},
	ConceptAnimation,
};

use crate::{
	characters::braidman::AnimationMenu,
	event::CharacterField,
	focus::{BODY_FOCUS, EYE_FOCUS, HEAD_ROOT_FOCUS, MOUTH_FOCUS},
};

#[derive(Clone, Debug, PartialEq)]
pub struct MygrHeadMenu {
	pub head: AssetSingleSelect<MygrHeadMesh>,
	pub skin: SwatchSingleSelect<MygrSkinColor>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MygrHeadFeaturesMenu {
	pub eye: AssetSingleSelect<EyeMesh>,
	pub snout: AssetSingleSelect<MygrMouthMesh>,
	pub eye_color: SwatchSingleSelect<MygrEyeColor>,
	pub mouth_color: SwatchSingleSelect<BraidmanColor>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MygrHairMenu {
	pub style: AssetSingleSelect<HairMesh>,
	pub color: SwatchSingleSelect<BraidmanColor>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MygrClothingMenu {
	pub layers: MultiSelect<ClothingMesh>,
	pub default_color: SwatchSingleSelect<BraidmanColor>,
	pub item_colors: Vec<ClothingColor>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MygrMenu {
	pub head: Section<MygrHeadMenu>,
	pub head_features: Section<MygrHeadFeaturesMenu>,
	pub hair: Section<MygrHairMenu>,
	pub clothing: Section<MygrClothingMenu>,
	pub animation: Section<AnimationMenu>,
}

impl From<&crozon_characters::species::mygr::MygrConfig> for MygrMenu {
	fn from(config: &crozon_characters::species::mygr::MygrConfig) -> Self {
		Self {
			head: Section::new(
				"Head",
				MygrHeadMenu {
					head: AssetSingleSelect::new(MygrHeadMesh::OrthoBear)
						.with_camera_focus(HEAD_ROOT_FOCUS),
					skin: SwatchSingleSelect::new(config.colors.skin),
				},
			),
			head_features: Section::new(
				"Head & Features",
				MygrHeadFeaturesMenu {
					eye: AssetSingleSelect::new(config.eye).with_camera_focus(EYE_FOCUS),
					snout: AssetSingleSelect::new(MygrMouthMesh::CanineSnout)
						.with_camera_focus(MOUTH_FOCUS),
					eye_color: SwatchSingleSelect::new(config.colors.eyes),
					mouth_color: SwatchSingleSelect::new(config.colors.mouth),
				},
			),
			hair: Section::new(
				"Hair",
				MygrHairMenu {
					style: AssetSingleSelect::new(config.hair).with_camera_focus(HEAD_ROOT_FOCUS),
					color: SwatchSingleSelect::new(config.colors.hair),
				},
			),
			clothing: Section::new(
				"Clothing",
				MygrClothingMenu {
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

impl From<&MygrMenu> for crozon_characters::species::mygr::MygrConfig {
	fn from(menu: &MygrMenu) -> Self {
		Self {
			eye: menu.head_features.value.eye.value,
			hair: menu.hair.value.style.value,
			clothing: menu.clothing.value.layers.selected.clone(),
			colors: crozon_characters::species::mygr::MygrColors {
				skin: menu.head.value.skin.value,
				eyes: menu.head_features.value.eye_color.value,
				mouth: menu.head_features.value.mouth_color.value,
				hair: menu.hair.value.color.value,
				clothing_default: menu.clothing.value.default_color.value,
				clothing: menu.clothing.value.item_colors.clone(),
			},
		}
	}
}

impl MygrMenu {
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
			CharacterField::MygrHead => self.head.value.head.camera_focus,
			CharacterField::Eye => self.head_features.value.eye.camera_focus,
			CharacterField::MygrMouth => self.head_features.value.snout.camera_focus,
			CharacterField::Hair => self.hair.value.style.camera_focus,
			CharacterField::Clothing(_) | CharacterField::Animation => Some(BODY_FOCUS),
			_ => None,
		}
	}
}

impl Default for MygrMenu {
	fn default() -> Self {
		Self::from(&crozon_characters::species::mygr::MygrConfig::default_preview())
	}
}
