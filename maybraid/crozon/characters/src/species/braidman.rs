//! Simplified Braidman species definition.
//!
//! This module is deliberately lean. It validates the organizational pattern for
//! species-owned assets, baseline proportions, presets, and slider resolution
//! before the full Braidman matrix is implemented.

pub mod assets;
pub mod pose;
pub mod presets;
pub mod sliders;

use crate::{
	presets::{BuildPreset, GenderPreset},
	species::SpeciesConfig,
	ResolvedCharacterAssembly,
};

use assets::BraidmanAssets;
use crate::species::common::{
	BodyMesh, ClothingMesh, EarMesh, EyeMesh, HairMesh, HeadMesh, MouthMesh, NoseMesh,
};
use sliders::BraidmanSliders;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum BraidmanColor {
	#[default]
	Natural,
	Warm,
	Cool,
	Dark,
	Light,
	Red,
	Blue,
	Green,
	Gold,
}

impl BraidmanColor {
	pub const fn label(self) -> &'static str {
		match self {
			Self::Natural => "natural",
			Self::Warm => "warm",
			Self::Cool => "cool",
			Self::Dark => "dark",
			Self::Light => "light",
			Self::Red => "red",
			Self::Blue => "blue",
			Self::Green => "green",
			Self::Gold => "gold",
		}
	}

	pub fn color(self) -> bevy::prelude::Color {
		match self {
			Self::Natural => bevy::prelude::Color::srgb(0.72, 0.54, 0.42),
			Self::Warm => bevy::prelude::Color::srgb(0.86, 0.58, 0.38),
			Self::Cool => bevy::prelude::Color::srgb(0.46, 0.60, 0.72),
			Self::Dark => bevy::prelude::Color::srgb(0.18, 0.16, 0.15),
			Self::Light => bevy::prelude::Color::srgb(0.88, 0.80, 0.68),
			Self::Red => bevy::prelude::Color::srgb(0.72, 0.18, 0.16),
			Self::Blue => bevy::prelude::Color::srgb(0.18, 0.30, 0.76),
			Self::Green => bevy::prelude::Color::srgb(0.22, 0.52, 0.28),
			Self::Gold => bevy::prelude::Color::srgb(0.88, 0.68, 0.22),
		}
	}
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClothingColor {
	pub clothing: ClothingMesh,
	pub color: BraidmanColor,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BraidmanColors {
	pub body: BraidmanColor,
	pub head: BraidmanColor,
	pub eyes: BraidmanColor,
	pub nose: BraidmanColor,
	pub mouth: BraidmanColor,
	pub ears: BraidmanColor,
	pub hair: BraidmanColor,
	pub clothing_default: BraidmanColor,
	pub clothing: Vec<ClothingColor>,
}

impl Default for BraidmanColors {
	fn default() -> Self {
		let body = BraidmanColor::Natural;
		Self {
			body,
			head: body,
			eyes: BraidmanColor::Blue,
			nose: body,
			mouth: BraidmanColor::Warm,
			ears: body,
			hair: BraidmanColor::Dark,
			clothing_default: BraidmanColor::Cool,
			clothing: Vec::new(),
		}
	}
}

impl BraidmanColors {
	pub fn skin_color(&self) -> BraidmanColor {
		self.body
	}

	pub fn sync_skin_from_body(&mut self) {
		let skin = self.body;
		self.head = skin;
		self.nose = skin;
		self.ears = skin;
	}

	pub fn clothing_color(&self, clothing: ClothingMesh) -> BraidmanColor {
		self.clothing
			.iter()
			.find(|choice| choice.clothing == clothing)
			.map(|choice| choice.color)
			.unwrap_or(self.clothing_default)
	}

	pub fn set_clothing_color(&mut self, clothing: ClothingMesh, color: BraidmanColor) {
		if let Some(choice) = self.clothing.iter_mut().find(|choice| choice.clothing == clothing) {
			choice.color = color;
		} else {
			self.clothing.push(ClothingColor { clothing, color });
		}
	}
}

/// Minimal unresolved Braidman state used by commands and, later, UI fields.
#[derive(Debug, Clone, PartialEq)]
pub struct BraidmanConfig {
	// Preset IDs are shared enums today; species-owned tables will live in `presets`.
	pub gender: GenderPreset,
	pub build: BuildPreset,
	pub body: BodyMesh,
	pub head: HeadMesh,
	pub eye: EyeMesh,
	pub nose: NoseMesh,
	pub mouth: MouthMesh,
	pub ear: EarMesh,
	pub hair: HairMesh,
	/// Multiple clothing layers compose; repeat `--clothing` on the CLI.
	pub clothing: Vec<ClothingMesh>,
	pub colors: BraidmanColors,
	pub sliders: BraidmanSliders,
}

impl Default for BraidmanConfig {
	fn default() -> Self {
		Self::default_preview()
	}
}

impl BraidmanConfig {
	/// Default command-driven preview, before any future UI field model exists.
	pub fn default_preview() -> Self {
		Self {
			gender: GenderPreset::Neutral,
			build: BuildPreset::Average,
			body: BodyMesh::Standard,
			head: HeadMesh::Standard,
			eye: EyeMesh::Standard,
			nose: NoseMesh::Standard,
			mouth: MouthMesh::Standard,
			ear: EarMesh::Standard,
			hair: HairMesh::None,
			clothing: Vec::new(),
			colors: BraidmanColors::default(),
			sliders: BraidmanSliders::default(),
		}
	}

	pub fn with_gender(mut self, gender: GenderPreset) -> Self {
		self.gender = gender;
		self
	}

	pub fn with_build(mut self, build: BuildPreset) -> Self {
		self.build = build;
		self
	}

	pub fn with_sliders(mut self, sliders: BraidmanSliders) -> Self {
		self.sliders = sliders;
		self
	}

	pub fn status_label(&self) -> String {
		let clothing = if self.clothing.is_empty() {
			"none".into()
		} else {
			self.clothing
				.iter()
				.map(|clothing| clothing.label())
				.collect::<Vec<_>>()
				.join(",")
		};
		format!(
			"braidman gender={} build={} body={} head={} eye={} nose={} mouth={} ear={} hair={} clothing={} colors=body:{} head:{} eyes:{} nose:{} mouth:{} ears:{} hair:{} sliders={}",
			self.gender.label(),
			self.build.label(),
			self.body.label(),
			self.head.label(),
			self.eye.label(),
			self.nose.label(),
			self.mouth.label(),
			self.ear.label(),
			self.hair.label(),
			clothing,
			self.colors.body.label(),
			self.colors.head.label(),
			self.colors.eyes.label(),
			self.colors.nose.label(),
			self.colors.mouth.label(),
			self.colors.ears.label(),
			self.colors.hair.label(),
			self.sliders.status_label(),
		)
	}

	pub fn sync_key(&self) -> String {
		format!("{self:?}")
	}
}

impl SpeciesConfig for BraidmanConfig {
	fn species_name(&self) -> &'static str {
		"braidman"
	}

	fn resolve(&self) -> ResolvedCharacterAssembly {
		BraidmanAssets::resolve(self)
	}
}
