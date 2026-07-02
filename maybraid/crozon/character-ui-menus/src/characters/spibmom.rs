use character_ui_menu::{AssetSingleSelect, CameraFocus, MultiSelect, Section, SwatchSingleSelect};
use crozon_characters::{
	species::{
		braidman::{BraidmanColor, ClothingColor},
		common::{ClothingMesh, EyeMesh, HairMesh},
		spibmom::{
			SpibmomCrownColor, SpibmomEarColor, SpibmomEyeColor, SpibmomHeadMesh, SpibmomMouthColor,
			SpibmomMouthMesh, SpibmomSkinColor, SpibmomSpineColor,
		},
	},
	ConceptAnimation,
};

use crate::{
	characters::braidman::AnimationMenu,
	event::CharacterField,
	focus::{BODY_FOCUS, CROWN_FOCUS, EYE_FOCUS, HEAD_ROOT_FOCUS, MOUTH_FOCUS},
};

#[derive(Clone, Debug, PartialEq)]
pub struct SpibmomHeadMenu {
	pub head: AssetSingleSelect<SpibmomHeadMesh>,
	pub skin: SwatchSingleSelect<SpibmomSkinColor>,
	pub crown_color: SwatchSingleSelect<SpibmomCrownColor>,
	pub spine_color: SwatchSingleSelect<SpibmomSpineColor>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SpibmomHeadFeaturesMenu {
	pub eye: AssetSingleSelect<EyeMesh>,
	pub snout: AssetSingleSelect<SpibmomMouthMesh>,
	pub eye_color: SwatchSingleSelect<SpibmomEyeColor>,
	pub ear_color: SwatchSingleSelect<SpibmomEarColor>,
	pub mouth_color: SwatchSingleSelect<SpibmomMouthColor>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SpibmomHairMenu {
	pub style: AssetSingleSelect<HairMesh>,
	pub color: SwatchSingleSelect<BraidmanColor>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SpibmomClothingMenu {
	pub layers: MultiSelect<ClothingMesh>,
	pub default_color: SwatchSingleSelect<BraidmanColor>,
	pub item_colors: Vec<ClothingColor>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SpibmomMenu {
	pub head: Section<SpibmomHeadMenu>,
	pub head_features: Section<SpibmomHeadFeaturesMenu>,
	pub hair: Section<SpibmomHairMenu>,
	pub clothing: Section<SpibmomClothingMenu>,
	pub animation: Section<AnimationMenu>,
}

impl From<&crozon_characters::species::spibmom::SpibmomConfig> for SpibmomMenu {
	fn from(config: &crozon_characters::species::spibmom::SpibmomConfig) -> Self {
		Self {
			head: Section::new(
				"Head",
				SpibmomHeadMenu {
					head: AssetSingleSelect::new(SpibmomHeadMesh::Meerkat)
						.with_camera_focus(HEAD_ROOT_FOCUS),
					skin: SwatchSingleSelect::new(config.colors.skin),
					crown_color: SwatchSingleSelect::new(config.colors.crown)
						.with_camera_focus(CROWN_FOCUS),
					spine_color: SwatchSingleSelect::new(config.colors.spine),
				},
			),
			head_features: Section::new(
				"Head & Features",
				SpibmomHeadFeaturesMenu {
					eye: AssetSingleSelect::new(config.eye).with_camera_focus(EYE_FOCUS),
					snout: AssetSingleSelect::new(SpibmomMouthMesh::Igny)
						.with_camera_focus(MOUTH_FOCUS),
					eye_color: SwatchSingleSelect::new(config.colors.eyes),
					ear_color: SwatchSingleSelect::new(config.colors.ears),
					mouth_color: SwatchSingleSelect::new(config.colors.mouth),
				},
			),
			hair: Section::new(
				"Hair",
				SpibmomHairMenu {
					style: AssetSingleSelect::new(config.hair).with_camera_focus(HEAD_ROOT_FOCUS),
					color: SwatchSingleSelect::new(config.colors.hair),
				},
			),
			clothing: Section::new(
				"Clothing",
				SpibmomClothingMenu {
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

impl From<&SpibmomMenu> for crozon_characters::species::spibmom::SpibmomConfig {
	fn from(menu: &SpibmomMenu) -> Self {
		Self {
			eye: menu.head_features.value.eye.value,
			hair: menu.hair.value.style.value,
			clothing: menu.clothing.value.layers.selected.clone(),
			colors: crozon_characters::species::spibmom::SpibmomColors {
				skin: menu.head.value.skin.value,
				eyes: menu.head_features.value.eye_color.value,
				ears: menu.head_features.value.ear_color.value,
				mouth: menu.head_features.value.mouth_color.value,
				crown: menu.head.value.crown_color.value,
				spine: menu.head.value.spine_color.value,
				hair: menu.hair.value.color.value,
				clothing_default: menu.clothing.value.default_color.value,
				clothing: menu.clothing.value.item_colors.clone(),
			},
		}
	}
}

impl SpibmomMenu {
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
			CharacterField::SpibmomHead => self.head.value.head.camera_focus,
			CharacterField::Eye => self.head_features.value.eye.camera_focus,
			CharacterField::SpibmomMouth => self.head_features.value.snout.camera_focus,
			CharacterField::SpibmomCrownColor => self.head.value.crown_color.camera_focus,
			CharacterField::Hair => self.hair.value.style.camera_focus,
			CharacterField::Clothing(_) | CharacterField::Animation | CharacterField::SpibmomSpineColor => {
				Some(BODY_FOCUS)
			}
			_ => None,
		}
	}
}

impl Default for SpibmomMenu {
	fn default() -> Self {
		Self::from(&crozon_characters::species::spibmom::SpibmomConfig::default_preview())
	}
}
