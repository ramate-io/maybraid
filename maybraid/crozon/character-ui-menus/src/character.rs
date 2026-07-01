use character_ui_menu::{
	AssetValue, CameraFocus, CharacterField, MenuEvent, Section, SectionId, SingleSelect,
	SwatchValue,
};
use crozon_characters::{
	species::{braidman::BraidmanConfig, brodler::BrodlerConfig},
	ConceptAnimation,
};

use crate::{braidman::BraidmanMenu, brodler::BrodlerMenu, cycle_value};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConceptSpecies {
	Braidman,
	Brodler,
}

impl ConceptSpecies {
	pub const fn label(self) -> &'static str {
		match self {
			Self::Braidman => "braidman",
			Self::Brodler => "brodler",
		}
	}
}

#[derive(Clone, Debug, PartialEq)]
pub enum SpeciesMenu {
	Braidman(Section<BraidmanMenu>),
	Brodler(Section<BrodlerMenu>),
}

#[derive(Clone, Debug, PartialEq)]
pub struct CharacterMenu {
	pub species: SingleSelect<ConceptSpecies>,
	pub braidman: BraidmanMenu,
	pub brodler: BrodlerMenu,
}

impl CharacterMenu {
	pub fn from_braidman(config: &BraidmanConfig, animation: ConceptAnimation) -> Self {
		Self {
			species: SingleSelect::new(ConceptSpecies::Braidman),
			braidman: BraidmanMenu::from(config).with_animation(animation),
			brodler: BrodlerMenu::default(),
		}
	}

	pub fn from_brodler(config: &BrodlerConfig, animation: ConceptAnimation) -> Self {
		Self {
			species: SingleSelect::new(ConceptSpecies::Brodler),
			braidman: BraidmanMenu::default(),
			brodler: BrodlerMenu::from(config).with_animation(animation),
		}
	}

	pub fn species_menu(&self) -> SpeciesMenu {
		match self.species.value {
			ConceptSpecies::Braidman => {
				SpeciesMenu::Braidman(Section::new("Braidman", self.braidman.clone()))
			}
			ConceptSpecies::Brodler => {
				SpeciesMenu::Brodler(Section::new("Brodler", self.brodler.clone()))
			}
		}
	}

	pub fn animation(&self) -> ConceptAnimation {
		match self.species.value {
			ConceptSpecies::Braidman => self.braidman.animation(),
			ConceptSpecies::Brodler => self.brodler.animation(),
		}
	}

	pub fn braidman_config(&self) -> BraidmanConfig {
		BraidmanConfig::from(&self.braidman)
	}

	pub fn brodler_config(&self) -> BrodlerConfig {
		BrodlerConfig::from(&self.brodler)
	}

	pub fn apply(&mut self, event: MenuEvent) -> bool {
		match self.species.value {
			ConceptSpecies::Braidman => self.apply_braidman(event),
			ConceptSpecies::Brodler => self.apply_brodler(event),
		}
	}

	pub fn camera_focus_for_event(&self, event: MenuEvent) -> Option<CameraFocus> {
		let field = match event {
			MenuEvent::SetAsset(field, _) | MenuEvent::Cycle(field, _) => Some(field),
			MenuEvent::ToggleClothing(clothing) => Some(CharacterField::Clothing(clothing)),
			_ => None,
		}?;
		match self.species.value {
			ConceptSpecies::Braidman => self.braidman.camera_focus_for_field(field),
			ConceptSpecies::Brodler => self.brodler.camera_focus_for_field(field),
		}
	}

	fn apply_braidman(&mut self, event: MenuEvent) -> bool {
		let menu = &mut self.braidman;
		match event {
			MenuEvent::ToggleSection(_) => false,
			MenuEvent::Cycle(CharacterField::Gender, delta) => {
				menu.presets.value.gender.value =
					cycle_value(menu.presets.value.gender.value, delta);
				true
			}
			MenuEvent::Cycle(CharacterField::Build, delta) => {
				menu.presets.value.build.value = cycle_value(menu.presets.value.build.value, delta);
				true
			}
			MenuEvent::SetAsset(field, value) => match (field, value) {
				(CharacterField::BodyMesh, AssetValue::Body(value)) => {
					menu.body.value.body.value = value;
					true
				}
				(CharacterField::HeadMesh, AssetValue::Head(value)) => {
					menu.head_features.value.head.value = value;
					true
				}
				(CharacterField::Eye, AssetValue::Eye(value)) => {
					menu.head_features.value.eye.value = value;
					true
				}
				(CharacterField::Nose, AssetValue::Nose(value)) => {
					menu.head_features.value.nose.value = value;
					true
				}
				(CharacterField::Mouth, AssetValue::Mouth(value)) => {
					menu.head_features.value.mouth.value = value;
					true
				}
				(CharacterField::Ear, AssetValue::Ear(value)) => {
					menu.head_features.value.ear.value = value;
					true
				}
				(CharacterField::Hair, AssetValue::Hair(value)) => {
					menu.hair.value.style.value = value;
					true
				}
				(CharacterField::Animation, AssetValue::Animation(value)) => {
					menu.animation.value.clip.value = value;
					true
				}
				_ => false,
			},
			MenuEvent::SliderDelta(field, delta) => apply_braidman_slider(menu, field, delta),
			MenuEvent::ToggleClothing(clothing) => {
				menu.clothing.value.layers.toggle(clothing);
				true
			}
			MenuEvent::SetSwatch(field, SwatchValue::Braidman(color)) => match field {
				CharacterField::BodyColor => {
					menu.body.value.color.value = color;
					true
				}
				CharacterField::EyeColor => {
					menu.head_features.value.eye_color.value = color;
					true
				}
				CharacterField::MouthColor => {
					menu.head_features.value.mouth_color.value = color;
					true
				}
				CharacterField::HairColor => {
					menu.hair.value.color.value = color;
					true
				}
				CharacterField::Clothing(clothing) => {
					menu.set_clothing_color(clothing, color);
					true
				}
				_ => false,
			},
			MenuEvent::SetSwatch(_, _) | MenuEvent::Cycle(_, _) => false,
		}
	}

	fn apply_brodler(&mut self, event: MenuEvent) -> bool {
		let menu = &mut self.brodler;
		match event {
			MenuEvent::ToggleSection(_) => false,
			MenuEvent::SetAsset(field, value) => match (field, value) {
				(CharacterField::BrodlerHead, AssetValue::BrodlerHead(value)) => {
					menu.head.value.head.value = value;
					true
				}
				(CharacterField::Horns, AssetValue::Horns(value)) => {
					menu.head.value.horns.value = value;
					true
				}
				(CharacterField::Eye, AssetValue::Eye(value)) => {
					menu.head_features.value.eye.value = value;
					true
				}
				(CharacterField::Nose, AssetValue::Nose(value)) => {
					menu.head_features.value.nose.value = value;
					true
				}
				(CharacterField::Mouth, AssetValue::Mouth(value)) => {
					menu.head_features.value.mouth.value = value;
					true
				}
				(CharacterField::Ear, AssetValue::Ear(value)) => {
					menu.head_features.value.ear.value = value;
					true
				}
				(CharacterField::Hair, AssetValue::Hair(value)) => {
					menu.hair.value.style.value = value;
					true
				}
				(CharacterField::Animation, AssetValue::Animation(value)) => {
					menu.animation.value.clip.value = value;
					true
				}
				_ => false,
			},
			MenuEvent::ToggleClothing(clothing) => {
				menu.clothing.value.layers.toggle(clothing);
				true
			}
			MenuEvent::SetSwatch(field, value) => match (field, value) {
				(CharacterField::SkinColor, SwatchValue::BrodlerSkin(color)) => {
					menu.head.value.skin.value = color;
					true
				}
				(CharacterField::BrodlerEyeColor, SwatchValue::BrodlerEye(color)) => {
					menu.head_features.value.eye_color.value = color;
					true
				}
				(CharacterField::HornColor, SwatchValue::BrodlerHorn(color)) => {
					menu.head_features.value.horn_color.value = color;
					true
				}
				(CharacterField::MouthColor, SwatchValue::Braidman(color)) => {
					menu.head_features.value.mouth_color.value = color;
					true
				}
				(CharacterField::HairColor, SwatchValue::Braidman(color)) => {
					menu.hair.value.color.value = color;
					true
				}
				(CharacterField::Clothing(clothing), SwatchValue::Braidman(color)) => {
					menu.set_clothing_color(clothing, color);
					true
				}
				_ => false,
			},
			MenuEvent::Cycle(_, _) | MenuEvent::SliderDelta(_, _) => false,
		}
	}
}

fn apply_braidman_slider(menu: &mut BraidmanMenu, field: CharacterField, delta: f32) -> bool {
	let sliders = &mut menu.body.value.sliders;
	match field {
		CharacterField::ShoulderWidth => {
			sliders.shoulder_width = sliders.shoulder_width.apply_delta(delta)
		}
		CharacterField::HipWidth => sliders.hip_width = sliders.hip_width.apply_delta(delta),
		CharacterField::ChestThickness => {
			sliders.chest_thickness = sliders.chest_thickness.apply_delta(delta)
		}
		CharacterField::HipThickness => {
			sliders.hip_thickness = sliders.hip_thickness.apply_delta(delta)
		}
		CharacterField::LegThickness => {
			sliders.leg_thickness = sliders.leg_thickness.apply_delta(delta)
		}
		CharacterField::ButtocksThickness => {
			sliders.buttocks_thickness = sliders.buttocks_thickness.apply_delta(delta)
		}
		CharacterField::WaistThickness => {
			sliders.waist_thickness = sliders.waist_thickness.apply_delta(delta)
		}
		CharacterField::LowerTrunkThickness => {
			sliders.lower_trunk_thickness = sliders.lower_trunk_thickness.apply_delta(delta)
		}
		CharacterField::ArmLength => sliders.arm_length = sliders.arm_length.apply_delta(delta),
		CharacterField::ArmThickness => {
			sliders.arm_thickness = sliders.arm_thickness.apply_delta(delta)
		}
		CharacterField::LegLength => sliders.leg_length = sliders.leg_length.apply_delta(delta),
		CharacterField::EyeWidth => sliders.eye_width = sliders.eye_width.apply_delta(delta),
		CharacterField::EyeHeight => sliders.eye_height = sliders.eye_height.apply_delta(delta),
		CharacterField::EyeTilt => sliders.eye_tilt = sliders.eye_tilt.apply_delta(delta),
		CharacterField::NoseWidth => sliders.nose_width = sliders.nose_width.apply_delta(delta),
		CharacterField::NoseHeight => sliders.nose_height = sliders.nose_height.apply_delta(delta),
		CharacterField::MouthWidth => sliders.mouth_width = sliders.mouth_width.apply_delta(delta),
		CharacterField::MouthHeight => {
			sliders.mouth_height = sliders.mouth_height.apply_delta(delta)
		}
		CharacterField::EarWidth => sliders.ear_width = sliders.ear_width.apply_delta(delta),
		CharacterField::EarHeight => sliders.ear_height = sliders.ear_height.apply_delta(delta),
		_ => return false,
	}
	true
}

impl Default for CharacterMenu {
	fn default() -> Self {
		Self::from_braidman(&BraidmanConfig::default_preview(), ConceptAnimation::default())
	}
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SectionOpenState {
	pub presets_open: bool,
	pub head_open: bool,
	pub body_open: bool,
	pub head_features_open: bool,
	pub hair_open: bool,
	pub clothing_open: bool,
	pub animation_open: bool,
}

impl SectionOpenState {
	pub fn is_open(self, section: SectionId) -> bool {
		match section {
			SectionId::Presets => self.presets_open,
			SectionId::Head => self.head_open,
			SectionId::Body => self.body_open,
			SectionId::HeadFeatures => self.head_features_open,
			SectionId::Hair => self.hair_open,
			SectionId::Clothing => self.clothing_open,
			SectionId::Animation => self.animation_open,
		}
	}

	pub fn toggle(&mut self, section: SectionId) {
		match section {
			SectionId::Presets => self.presets_open = !self.presets_open,
			SectionId::Head => self.head_open = !self.head_open,
			SectionId::Body => self.body_open = !self.body_open,
			SectionId::HeadFeatures => self.head_features_open = !self.head_features_open,
			SectionId::Hair => self.hair_open = !self.hair_open,
			SectionId::Clothing => self.clothing_open = !self.clothing_open,
			SectionId::Animation => self.animation_open = !self.animation_open,
		}
	}
}

impl Default for SectionOpenState {
	fn default() -> Self {
		Self {
			presets_open: true,
			head_open: true,
			body_open: true,
			head_features_open: false,
			hair_open: false,
			clothing_open: true,
			animation_open: false,
		}
	}
}
