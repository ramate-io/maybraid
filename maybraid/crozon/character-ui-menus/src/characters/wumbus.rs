use character_ui_menu::{AssetSingleSelect, CameraFocus, MultiSelect, Section, SingleSelect, SwatchSingleSelect};
use crozon_characters::{
	species::{
		braidman::{BraidmanColor, ClothingColor},
		common::{ClothingMesh, EyeMesh, HairMesh},
		wumbus::{
			WumbusEarColor, WumbusEyeColor, WumbusHeadMesh, WumbusHornColor, WumbusHornMesh,
			WumbusMouthColor, WumbusMouthMesh, WumbusSkinColor, WumbusSpineColor,
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
pub struct WumbusHeadMenu {
	pub head: AssetSingleSelect<WumbusHeadMesh>,
	pub skin: SwatchSingleSelect<WumbusSkinColor>,
	pub horns: SingleSelect<WumbusHornMesh>,
	pub horn_color: SwatchSingleSelect<WumbusHornColor>,
	pub spine_color: SwatchSingleSelect<WumbusSpineColor>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct WumbusHeadFeaturesMenu {
	pub eye: AssetSingleSelect<EyeMesh>,
	pub snout: AssetSingleSelect<WumbusMouthMesh>,
	pub eye_color: SwatchSingleSelect<WumbusEyeColor>,
	pub ear_color: SwatchSingleSelect<WumbusEarColor>,
	pub mouth_color: SwatchSingleSelect<WumbusMouthColor>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct WumbusHairMenu {
	pub style: AssetSingleSelect<HairMesh>,
	pub color: SwatchSingleSelect<BraidmanColor>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct WumbusClothingMenu {
	pub layers: MultiSelect<ClothingMesh>,
	pub default_color: SwatchSingleSelect<BraidmanColor>,
	pub item_colors: Vec<ClothingColor>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct WumbusMenu {
	pub head: Section<WumbusHeadMenu>,
	pub head_features: Section<WumbusHeadFeaturesMenu>,
	pub hair: Section<WumbusHairMenu>,
	pub clothing: Section<WumbusClothingMenu>,
	pub animation: Section<AnimationMenu>,
}

impl From<&crozon_characters::species::wumbus::WumbusConfig> for WumbusMenu {
	fn from(config: &crozon_characters::species::wumbus::WumbusConfig) -> Self {
		Self {
			head: Section::new(
				"Head",
				WumbusHeadMenu {
					head: AssetSingleSelect::new(WumbusHeadMesh::OrthoBear)
						.with_camera_focus(HEAD_ROOT_FOCUS),
					skin: SwatchSingleSelect::new(config.colors.skin),
					horns: SingleSelect::new(config.horns).with_camera_focus(CROWN_FOCUS),
					horn_color: SwatchSingleSelect::new(config.colors.horns),
					spine_color: SwatchSingleSelect::new(config.colors.spine),
				},
			),
			head_features: Section::new(
				"Head & Features",
				WumbusHeadFeaturesMenu {
					eye: AssetSingleSelect::new(config.eye).with_camera_focus(EYE_FOCUS),
					snout: AssetSingleSelect::new(WumbusMouthMesh::CanineSnout)
						.with_camera_focus(MOUTH_FOCUS),
					eye_color: SwatchSingleSelect::new(config.colors.eyes),
					ear_color: SwatchSingleSelect::new(config.colors.ears),
					mouth_color: SwatchSingleSelect::new(config.colors.mouth),
				},
			),
			hair: Section::new(
				"Hair",
				WumbusHairMenu {
					style: AssetSingleSelect::new(config.hair).with_camera_focus(HEAD_ROOT_FOCUS),
					color: SwatchSingleSelect::new(config.colors.hair),
				},
			),
			clothing: Section::new(
				"Clothing",
				WumbusClothingMenu {
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

impl From<&WumbusMenu> for crozon_characters::species::wumbus::WumbusConfig {
	fn from(menu: &WumbusMenu) -> Self {
		Self {
			horns: menu.head.value.horns.value,
			eye: menu.head_features.value.eye.value,
			hair: menu.hair.value.style.value,
			clothing: menu.clothing.value.layers.selected.clone(),
			colors: crozon_characters::species::wumbus::WumbusColors {
				skin: menu.head.value.skin.value,
				eyes: menu.head_features.value.eye_color.value,
				ears: menu.head_features.value.ear_color.value,
				mouth: menu.head_features.value.mouth_color.value,
				horns: menu.head.value.horn_color.value,
				spine: menu.head.value.spine_color.value,
				hair: menu.hair.value.color.value,
				clothing_default: menu.clothing.value.default_color.value,
				clothing: menu.clothing.value.item_colors.clone(),
			},
		}
	}
}

impl WumbusMenu {
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
			CharacterField::WumbusHead => self.head.value.head.camera_focus,
			CharacterField::WumbusHorns => {
				if self.head.value.horns.value == WumbusHornMesh::None {
					self.head.value.head.camera_focus
				} else {
					self.head.value.horns.camera_focus
				}
			}
			CharacterField::Eye => self.head_features.value.eye.camera_focus,
			CharacterField::WumbusMouth => self.head_features.value.snout.camera_focus,
			CharacterField::Hair => self.hair.value.style.camera_focus,
			CharacterField::Clothing(_) | CharacterField::Animation => Some(BODY_FOCUS),
			_ => None,
		}
	}
}

impl Default for WumbusMenu {
	fn default() -> Self {
		Self::from(&crozon_characters::species::wumbus::WumbusConfig::default_preview())
	}
}
