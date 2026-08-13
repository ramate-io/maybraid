//! Mygr species definition.
//!
//! Catlike humanoid: ortho bear head on the Leron full body, canine snout, flank
//! ears, species-owned fur/eye colors, and shared hair/clothing catalogs.

pub mod assets;
pub mod recipe;
pub use recipe::Mygr;
pub mod palette;
pub mod pose;

use crate::{
	species::common::{EyeMesh, HairMesh},
	CharacterRecipe, Clothed, ClothingLayer,
};

use crozon_character_items::{ClothingColor, ClothingMesh, ItemColor};

pub use assets::{MygrHeadMesh, MygrMouthMesh};
pub use palette::{MygrEyeColor, MygrSkinColor};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MygrColors {
	pub skin: MygrSkinColor,
	pub eyes: MygrEyeColor,
	pub mouth: ItemColor,
	pub hair: ItemColor,
	pub clothing_default: ItemColor,
	pub clothing: Vec<ClothingColor>,
}

impl Default for MygrColors {
	fn default() -> Self {
		Self {
			skin: MygrSkinColor::Ginger,
			eyes: MygrEyeColor::Green,
			mouth: ItemColor::Natural,
			hair: ItemColor::Dark,
			clothing_default: ItemColor::Cool,
			clothing: Vec::new(),
		}
	}
}

impl MygrColors {
	pub fn color_for_slot(&self, slot: crate::CharacterPartSlot) -> bevy::prelude::Color {
		use crate::CharacterPartSlot::*;
		match slot {
			BodyMesh | HeadMesh | HeadRig | EarLeft | EarRight | Tail => self.skin.color(),
			EyeLeft | EyeRight => self.eyes.color(),
			Mouth => self.mouth.color(),
			Hair => self.hair.color(),
			_ => self.skin.color(),
		}
	}

	pub fn clothing_color(&self, clothing: ClothingMesh) -> ItemColor {
		ClothingColor::resolve(&self.clothing, self.clothing_default, clothing)
	}

	pub fn set_clothing_color(&mut self, clothing: ClothingMesh, color: ItemColor) {
		ClothingColor::set(&mut self.clothing, clothing, color);
	}
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MygrConfig {
	pub eye: EyeMesh,
	pub hair: HairMesh,
	pub clothing: Vec<ClothingMesh>,
	pub colors: MygrColors,
}

impl Default for MygrConfig {
	fn default() -> Self {
		Self::default_preview()
	}
}

impl MygrConfig {
	pub fn default_preview() -> Self {
		Self {
			eye: EyeMesh::Standard,
			hair: HairMesh::None,
			clothing: Vec::new(),
			colors: MygrColors::default(),
		}
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
			"mygr eye={} hair={} clothing={} skin={} eyes={} hair_color={}",
			self.eye.label(),
			self.hair.label(),
			clothing,
			self.colors.skin.label(),
			self.colors.eyes.label(),
			self.colors.hair.label(),
		)
	}

	pub fn sync_key(&self) -> String {
		format!("{self:?}")
	}

	/// Inner recipe plus clothing layers (`Clothed<Mygr>`).
	pub fn clothed(&self) -> Clothed<Mygr> {
		CharacterRecipe::clothed(self)
	}
}

impl CharacterRecipe for MygrConfig {
	type Components = Mygr;

	fn components(&self) -> Self::Components {
		Mygr::from_config(self)
	}

	fn clothing_layers(&self) -> Vec<ClothingLayer> {
		crate::clothing_layers(self.clothing.iter().copied(), |mesh| {
			self.colors.clothing_color(mesh)
		})
	}
}
