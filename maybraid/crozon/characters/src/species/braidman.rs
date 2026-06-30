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

use assets::{
	BodyMesh, BraidmanAssets, ClothingMesh, EarMesh, EyeMesh, HairMesh, HeadMesh, MouthMesh,
	NoseMesh,
};
use sliders::BraidmanSliders;

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
			"braidman gender={} build={} body={} head={} eye={} nose={} mouth={} ear={} hair={} clothing={} sliders={}",
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
