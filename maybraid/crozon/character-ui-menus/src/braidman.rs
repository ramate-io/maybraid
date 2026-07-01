use character_ui_menu::{
	AssetSingleSelect, CameraFocus, CharacterField, MultiSelect, Section, SingleSelect, Slider,
	SwatchSingleSelect,
};
use crozon_characters::{
	presets::{BuildPreset, GenderPreset},
	species::{
		braidman::{
			sliders::BraidmanSliders, BraidmanColor, BraidmanColors, BraidmanConfig, ClothingColor,
		},
		common::{
			BodyMesh, ClothingMesh, EarMesh, EyeMesh, HairMesh, HeadMesh, MouthMesh, NoseMesh,
		},
	},
	ConceptAnimation,
};

use crate::focus::{
	BODY_FOCUS, CROWN_FOCUS, EAR_FOCUS, EYE_FOCUS, HEAD_ROOT_FOCUS, MOUTH_FOCUS, NOSE_FOCUS,
};

#[derive(Clone, Debug, PartialEq)]
pub struct BraidmanPresetsMenu {
	pub gender: SingleSelect<GenderPreset>,
	pub build: SingleSelect<BuildPreset>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BraidmanSlidersMenu {
	pub shoulder_width: Slider,
	pub hip_width: Slider,
	pub chest_thickness: Slider,
	pub hip_thickness: Slider,
	pub leg_thickness: Slider,
	pub buttocks_thickness: Slider,
	pub waist_thickness: Slider,
	pub lower_trunk_thickness: Slider,
	pub arm_length: Slider,
	pub arm_thickness: Slider,
	pub leg_length: Slider,
	pub eye_width: Slider,
	pub eye_height: Slider,
	pub eye_tilt: Slider,
	pub nose_width: Slider,
	pub nose_height: Slider,
	pub mouth_width: Slider,
	pub mouth_height: Slider,
	pub ear_width: Slider,
	pub ear_height: Slider,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BraidmanBodyMenu {
	pub body: AssetSingleSelect<BodyMesh>,
	pub sliders: BraidmanSlidersMenu,
	pub color: SwatchSingleSelect<BraidmanColor>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BraidmanHeadFeaturesMenu {
	pub head: AssetSingleSelect<HeadMesh>,
	pub eye: AssetSingleSelect<EyeMesh>,
	pub nose: AssetSingleSelect<NoseMesh>,
	pub mouth: AssetSingleSelect<MouthMesh>,
	pub ear: AssetSingleSelect<EarMesh>,
	pub eye_color: SwatchSingleSelect<BraidmanColor>,
	pub mouth_color: SwatchSingleSelect<BraidmanColor>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BraidmanHairMenu {
	pub style: AssetSingleSelect<HairMesh>,
	pub color: SwatchSingleSelect<BraidmanColor>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BraidmanClothingMenu {
	pub layers: MultiSelect<ClothingMesh>,
	pub default_color: SwatchSingleSelect<BraidmanColor>,
	pub item_colors: Vec<ClothingColor>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AnimationMenu {
	pub clip: AssetSingleSelect<ConceptAnimation>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BraidmanMenu {
	pub presets: Section<BraidmanPresetsMenu>,
	pub body: Section<BraidmanBodyMenu>,
	pub head_features: Section<BraidmanHeadFeaturesMenu>,
	pub hair: Section<BraidmanHairMenu>,
	pub clothing: Section<BraidmanClothingMenu>,
	pub animation: Section<AnimationMenu>,
}

fn slider(value: f32, min: f32, max: f32, step: f32) -> Slider {
	Slider::new(value, min, max, step)
}

impl BraidmanSlidersMenu {
	pub fn from_config(sliders: BraidmanSliders) -> Self {
		Self {
			shoulder_width: slider(sliders.shoulder_width, 0.8, 1.2, 0.05),
			hip_width: slider(sliders.hip_width, 0.8, 1.4, 0.05),
			chest_thickness: slider(sliders.chest_thickness, 0.8, 1.2, 0.05),
			hip_thickness: slider(sliders.hip_thickness, 0.8, 1.2, 0.05),
			leg_thickness: slider(sliders.leg_thickness, 0.8, 1.2, 0.05),
			buttocks_thickness: slider(sliders.buttocks_thickness, 0.8, 1.2, 0.05),
			waist_thickness: slider(sliders.waist_thickness, 0.8, 1.2, 0.05),
			lower_trunk_thickness: slider(sliders.lower_trunk_thickness, 0.8, 1.2, 0.05),
			arm_length: slider(sliders.arm_length, 0.8, 1.2, 0.05),
			arm_thickness: slider(sliders.arm_thickness, 0.8, 1.2, 0.05),
			leg_length: slider(sliders.leg_length, 0.8, 1.2, 0.05),
			eye_width: slider(sliders.eye_width, 0.8, 1.2, 0.05),
			eye_height: slider(sliders.eye_height, 0.8, 1.2, 0.05),
			eye_tilt: slider(sliders.eye_tilt, -30.0, 30.0, 0.5),
			nose_width: slider(sliders.nose_width, 0.8, 1.2, 0.05),
			nose_height: slider(sliders.nose_height, 0.8, 1.2, 0.05),
			mouth_width: slider(sliders.mouth_width, 0.8, 1.2, 0.05),
			mouth_height: slider(sliders.mouth_height, 0.8, 1.2, 0.05),
			ear_width: slider(sliders.ear_width, 0.8, 1.2, 0.05),
			ear_height: slider(sliders.ear_height, 0.8, 1.2, 0.05),
		}
	}

	pub fn to_config(&self) -> BraidmanSliders {
		BraidmanSliders {
			shoulder_width: self.shoulder_width.value,
			hip_width: self.hip_width.value,
			chest_thickness: self.chest_thickness.value,
			hip_thickness: self.hip_thickness.value,
			leg_thickness: self.leg_thickness.value,
			buttocks_thickness: self.buttocks_thickness.value,
			waist_thickness: self.waist_thickness.value,
			lower_trunk_thickness: self.lower_trunk_thickness.value,
			arm_length: self.arm_length.value,
			arm_thickness: self.arm_thickness.value,
			leg_length: self.leg_length.value,
			eye_width: self.eye_width.value,
			eye_height: self.eye_height.value,
			eye_tilt: self.eye_tilt.value,
			nose_width: self.nose_width.value,
			nose_height: self.nose_height.value,
			mouth_width: self.mouth_width.value,
			mouth_height: self.mouth_height.value,
			ear_width: self.ear_width.value,
			ear_height: self.ear_height.value,
		}
		.clamped()
	}
}

impl From<&BraidmanConfig> for BraidmanMenu {
	fn from(config: &BraidmanConfig) -> Self {
		Self {
			presets: Section::new(
				"Presets",
				BraidmanPresetsMenu {
					gender: SingleSelect::new(config.gender),
					build: SingleSelect::new(config.build),
				},
			),
			body: Section::new(
				"Body",
				BraidmanBodyMenu {
					body: AssetSingleSelect::new(config.body).with_camera_focus(BODY_FOCUS),
					sliders: BraidmanSlidersMenu::from_config(config.sliders),
					color: SwatchSingleSelect::new(config.colors.body),
				},
			)
			.with_camera_focus(BODY_FOCUS),
			head_features: Section::new(
				"Head & Features",
				BraidmanHeadFeaturesMenu {
					head: AssetSingleSelect::new(config.head).with_camera_focus(HEAD_ROOT_FOCUS),
					eye: AssetSingleSelect::new(config.eye).with_camera_focus(EYE_FOCUS),
					nose: AssetSingleSelect::new(config.nose).with_camera_focus(NOSE_FOCUS),
					mouth: AssetSingleSelect::new(config.mouth).with_camera_focus(MOUTH_FOCUS),
					ear: AssetSingleSelect::new(config.ear).with_camera_focus(EAR_FOCUS),
					eye_color: SwatchSingleSelect::new(config.colors.eyes),
					mouth_color: SwatchSingleSelect::new(config.colors.mouth),
				},
			),
			hair: Section::new(
				"Hair",
				BraidmanHairMenu {
					style: AssetSingleSelect::new(config.hair).with_camera_focus(CROWN_FOCUS),
					color: SwatchSingleSelect::new(config.colors.hair),
				},
			),
			clothing: Section::new(
				"Clothing",
				BraidmanClothingMenu {
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

impl From<&BraidmanMenu> for BraidmanConfig {
	fn from(menu: &BraidmanMenu) -> Self {
		let body_color = menu.body.value.color.value;
		Self {
			gender: menu.presets.value.gender.value,
			build: menu.presets.value.build.value,
			body: menu.body.value.body.value,
			head: menu.head_features.value.head.value,
			eye: menu.head_features.value.eye.value,
			nose: menu.head_features.value.nose.value,
			mouth: menu.head_features.value.mouth.value,
			ear: menu.head_features.value.ear.value,
			hair: menu.hair.value.style.value,
			clothing: menu.clothing.value.layers.selected.clone(),
			colors: BraidmanColors {
				body: body_color,
				head: body_color,
				eyes: menu.head_features.value.eye_color.value,
				nose: body_color,
				mouth: menu.head_features.value.mouth_color.value,
				ears: body_color,
				hair: menu.hair.value.color.value,
				clothing_default: menu.clothing.value.default_color.value,
				clothing: menu.clothing.value.item_colors.clone(),
			},
			sliders: menu.body.value.sliders.to_config(),
		}
	}
}

impl BraidmanMenu {
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
			CharacterField::BodyMesh => self.body.value.body.camera_focus,
			CharacterField::HeadMesh => self.head_features.value.head.camera_focus,
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

impl Default for BraidmanMenu {
	fn default() -> Self {
		Self::from(&BraidmanConfig::default_preview())
	}
}
