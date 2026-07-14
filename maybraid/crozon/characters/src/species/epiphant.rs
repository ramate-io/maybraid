//! Epiphant species definition.
//!
//! Elephant-like quadruped: Epiphant body on the quadruped rig, orthograde
//! meerkat head, Epiphant ears, trunkish nose, and cat tail.

pub mod assets;
pub mod bsn;
pub mod pose;
pub mod presets;
pub mod sliders;

use crate::{
	presets::{BuildPreset, GenderPreset},
	species::SpeciesConfig,
	ResolvedCharacterAssembly,
};

use crate::species::common::EyeMesh;
use assets::EpiphantAssets;
use crozon_character_items::ItemColor;
use sliders::EpiphantSliders;

pub use assets::{EpiphantBodyMesh, EpiphantEarMesh, EpiphantHeadMesh, EpiphantNoseMesh};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EpiphantColors {
	pub body: ItemColor,
	pub head: ItemColor,
	pub eyes: ItemColor,
	pub ears: ItemColor,
	pub nose: ItemColor,
	pub tail: ItemColor,
}

impl Default for EpiphantColors {
	fn default() -> Self {
		let body = ItemColor::Natural;
		Self {
			body,
			head: body,
			eyes: ItemColor::Blue,
			ears: body,
			nose: ItemColor::Natural,
			tail: body,
		}
	}
}

impl EpiphantColors {
	pub fn skin_color(&self) -> ItemColor {
		self.body
	}

	pub fn sync_skin_from_body(&mut self) {
		let skin = self.body;
		self.head = skin;
		self.ears = skin;
		self.tail = skin;
	}
}

#[derive(Debug, Clone, PartialEq)]
pub struct EpiphantConfig {
	pub gender: GenderPreset,
	pub build: BuildPreset,
	pub body: EpiphantBodyMesh,
	pub head: EpiphantHeadMesh,
	pub ear: EpiphantEarMesh,
	pub nose: EpiphantNoseMesh,
	pub eye: EyeMesh,
	pub colors: EpiphantColors,
	pub sliders: EpiphantSliders,
}

impl Default for EpiphantConfig {
	fn default() -> Self {
		Self::default_preview()
	}
}

impl EpiphantConfig {
	pub fn default_preview() -> Self {
		Self {
			gender: GenderPreset::Neutral,
			build: BuildPreset::Average,
			body: EpiphantBodyMesh::Epiphant,
			head: EpiphantHeadMesh::Meerkat,
			ear: EpiphantEarMesh::Epiphant,
			nose: EpiphantNoseMesh::Trunkish,
			eye: EyeMesh::Standard,
			colors: EpiphantColors::default(),
			sliders: EpiphantSliders::default(),
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

	pub fn with_sliders(mut self, sliders: EpiphantSliders) -> Self {
		self.sliders = sliders;
		self
	}

	pub fn status_label(&self) -> String {
		format!(
			"epiphant gender={} build={} body={} head={} ear={} nose={} eye={} colors=body:{} head:{} eyes:{} ears:{} nose:{} tail:{} sliders={}",
			self.gender.label(),
			self.build.label(),
			self.body.label(),
			self.head.label(),
			self.ear.label(),
			self.nose.label(),
			self.eye.label(),
			self.colors.body.label(),
			self.colors.head.label(),
			self.colors.eyes.label(),
			self.colors.ears.label(),
			self.colors.nose.label(),
			self.colors.tail.label(),
			self.sliders.status_label(),
		)
	}

	pub fn sync_key(&self) -> String {
		format!("{self:?}")
	}
}

impl SpeciesConfig for EpiphantConfig {
	fn species_name(&self) -> &'static str {
		"epiphant"
	}

	fn resolve(&self) -> ResolvedCharacterAssembly {
		EpiphantAssets::resolve(self)
	}
}
