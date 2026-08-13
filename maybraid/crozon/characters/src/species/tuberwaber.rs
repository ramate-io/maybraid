//! Tuberwaber species definition.
//!
//! Biped similar to Braidman: humanoid rig with the tuberwaber body and head on
//! an orthograde head stack, shared features (eyes/nose/mouth/hair/clothing), a
//! fixed harrowed crown, and a colorful cool-toned skin palette (no ears).

pub mod assets;
pub mod bsn;
pub mod palette;
pub mod pose;
pub mod presets;
pub mod sliders;

use crate::{
	presets::{BuildPreset, GenderPreset},
	CharacterRecipe, Clothed, ClothingLayer,
};

use crate::species::common::{EyeMesh, HairMesh, MouthMesh, NoseMesh};
use crozon_character_items::{ClothingColor, ClothingMesh, ItemColor};
use sliders::TuberwaberSliders;

pub use assets::{TuberwaberBodyMesh, TuberwaberHeadMesh};
pub use palette::TuberwaberColor;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TuberwaberColors {
	pub body: TuberwaberColor,
	pub head: TuberwaberColor,
	pub eyes: TuberwaberColor,
	pub nose: TuberwaberColor,
	pub mouth: TuberwaberColor,
	pub horns: TuberwaberColor,
	pub hair: ItemColor,
	pub clothing_default: ItemColor,
	pub clothing: Vec<ClothingColor>,
}

impl Default for TuberwaberColors {
	fn default() -> Self {
		let body = TuberwaberColor::MistBlue;
		Self {
			body,
			head: body,
			eyes: TuberwaberColor::Teal,
			nose: body,
			mouth: TuberwaberColor::Coral,
			horns: TuberwaberColor::Slate,
			hair: ItemColor::Dark,
			clothing_default: ItemColor::Cool,
			clothing: Vec::new(),
		}
	}
}

impl TuberwaberColors {
	pub fn skin_color(&self) -> TuberwaberColor {
		self.body
	}

	pub fn sync_skin_from_body(&mut self) {
		let skin = self.body;
		self.head = skin;
		self.nose = skin;
	}

	pub fn clothing_color(&self, clothing: ClothingMesh) -> ItemColor {
		ClothingColor::resolve(&self.clothing, self.clothing_default, clothing)
	}

	pub fn set_clothing_color(&mut self, clothing: ClothingMesh, color: ItemColor) {
		ClothingColor::set(&mut self.clothing, clothing, color);
	}
}

#[derive(Debug, Clone, PartialEq)]
pub struct TuberwaberConfig {
	pub gender: GenderPreset,
	pub build: BuildPreset,
	pub body: TuberwaberBodyMesh,
	pub head: TuberwaberHeadMesh,
	pub eye: EyeMesh,
	pub nose: NoseMesh,
	pub mouth: MouthMesh,
	pub hair: HairMesh,
	pub clothing: Vec<ClothingMesh>,
	pub colors: TuberwaberColors,
	pub sliders: TuberwaberSliders,
}

impl Default for TuberwaberConfig {
	fn default() -> Self {
		Self::default_preview()
	}
}

impl TuberwaberConfig {
	pub fn default_preview() -> Self {
		Self {
			gender: GenderPreset::Neutral,
			build: BuildPreset::Average,
			body: TuberwaberBodyMesh::Tuberwaber,
			head: TuberwaberHeadMesh::Tuberwaber,
			eye: EyeMesh::Standard,
			nose: NoseMesh::Loaf,
			mouth: MouthMesh::Standard,
			hair: HairMesh::None,
			clothing: Vec::new(),
			colors: TuberwaberColors::default(),
			sliders: TuberwaberSliders::default(),
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

	pub fn with_sliders(mut self, sliders: TuberwaberSliders) -> Self {
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
			"tuberwaber gender={} build={} body={} head={} eye={} nose={} mouth={} hair={} clothing={} colors=body:{} head:{} eyes:{} nose:{} mouth:{} horns:{} hair:{} sliders={}",
			self.gender.label(),
			self.build.label(),
			self.body.label(),
			self.head.label(),
			self.eye.label(),
			self.nose.label(),
			self.mouth.label(),
			self.hair.label(),
			clothing,
			self.colors.body.label(),
			self.colors.head.label(),
			self.colors.eyes.label(),
			self.colors.nose.label(),
			self.colors.mouth.label(),
			self.colors.horns.label(),
			self.colors.hair.label(),
			self.sliders.status_label(),
		)
	}

	pub fn sync_key(&self) -> String {
		format!("{self:?}")
	}

	/// Inner recipe plus clothing layers (`Clothed<Tuberwaber>`).
	pub fn clothed(&self) -> Clothed<crate::species::tuberwaber::bsn::Tuberwaber> {
		CharacterRecipe::clothed(self)
	}
}

impl CharacterRecipe for TuberwaberConfig {
	type Components = crate::species::tuberwaber::bsn::Tuberwaber;

	fn components(&self) -> Self::Components {
		crate::species::tuberwaber::bsn::Tuberwaber::from_config(self)
	}

	fn clothing_layers(&self) -> Vec<ClothingLayer> {
		crate::clothing_layers(self.clothing.iter().copied(), |mesh| {
			self.colors.clothing_color(mesh)
		})
	}
}
