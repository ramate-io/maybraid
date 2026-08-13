//! Caole species definition.
//!
//! Quadruped grazer: Gumbus or Rumbler body on the quadruped rig, Cowder head,
//! flank ears, cat tail, and cow snout. Head mesh is Caole or Cowder.

pub mod assets;
pub mod bsn;
pub mod pose;
pub mod presets;
pub mod sliders;

use crate::{
	presets::{BuildPreset, GenderPreset},
	species::SpeciesConfig,
	CharacterRecipe, Clothed, ClothingLayer, ResolvedCharacterAssembly,
};

use crate::species::common::EyeMesh;
use assets::CaoleAssets;
use crozon_character_items::ItemColor;
use sliders::CaoleSliders;

pub use assets::{CaoleBodyMesh, CaoleMouthMesh};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaoleColors {
	pub body: ItemColor,
	pub head: ItemColor,
	pub eyes: ItemColor,
	pub ears: ItemColor,
	pub mouth: ItemColor,
	pub tail: ItemColor,
}

impl Default for CaoleColors {
	fn default() -> Self {
		let body = ItemColor::Natural;
		Self {
			body,
			head: body,
			eyes: ItemColor::Blue,
			ears: body,
			mouth: ItemColor::Natural,
			tail: body,
		}
	}
}

impl CaoleColors {
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
pub struct CaoleConfig {
	pub gender: GenderPreset,
	pub build: BuildPreset,
	pub body: CaoleBodyMesh,
	pub mouth: CaoleMouthMesh,
	pub eye: EyeMesh,
	pub colors: CaoleColors,
	pub sliders: CaoleSliders,
}

impl Default for CaoleConfig {
	fn default() -> Self {
		Self::default_preview()
	}
}

impl CaoleConfig {
	pub fn default_preview() -> Self {
		Self {
			gender: GenderPreset::Neutral,
			build: BuildPreset::Average,
			body: CaoleBodyMesh::Gumbus,
			mouth: CaoleMouthMesh::Cow,
			eye: EyeMesh::Standard,
			colors: CaoleColors::default(),
			sliders: CaoleSliders::default(),
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

	pub fn with_sliders(mut self, sliders: CaoleSliders) -> Self {
		self.sliders = sliders;
		self
	}

	pub fn status_label(&self) -> String {
		format!(
			"caole gender={} build={} body={} mouth={} eye={} colors=body:{} head:{} eyes:{} ears:{} mouth:{} tail:{} sliders={}",
			self.gender.label(),
			self.build.label(),
			self.body.label(),
			self.mouth.label(),
			self.eye.label(),
			self.colors.body.label(),
			self.colors.head.label(),
			self.colors.eyes.label(),
			self.colors.ears.label(),
			self.colors.mouth.label(),
			self.colors.tail.label(),
			self.sliders.status_label(),
		)
	}

	pub fn sync_key(&self) -> String {
		format!("{self:?}")
	}

	/// Inner recipe plus empty clothing layers (`Clothed<Caole>`).
	pub fn clothed(&self) -> Clothed<crate::species::caole::bsn::Caole> {
		CharacterRecipe::clothed(self)
	}
}

impl CharacterRecipe for CaoleConfig {
	type Components = crate::species::caole::bsn::Caole;

	fn components(&self) -> Self::Components {
		crate::species::caole::bsn::Caole::from_config(self)
	}

	fn clothing_layers(&self) -> Vec<ClothingLayer> {
		Vec::new()
	}
}

impl SpeciesConfig for CaoleConfig {
	fn species_name(&self) -> &'static str {
		"caole"
	}

	fn resolve(&self) -> ResolvedCharacterAssembly {
		CaoleAssets::resolve(self)
	}
}
