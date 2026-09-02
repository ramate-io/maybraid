//! Brokker species definition.
//!
//! Pterodactyl-like biped: libird body on the humanoid rig at full stature, ortho
//! tee head, fixed igny snout in the mouth slot, and a wide wingspan via arm bone
//! proportion layers (no overall asset normalization).

pub mod assets;
pub mod recipe;
pub use recipe::Brokker;
pub mod palette;
pub mod pose;

use crate::{
	species::common::{EyeMesh, HairMesh},
	CharacterRecipe, ClothingLayer,
};

use crozon_character_items::{
	ClothingColor, ClothingHost, ClothingMaterial, ClothingMesh, ItemColor,
};

pub use assets::{BrokkerHeadMesh, BrokkerSnoutMesh};
pub use palette::{BrokkerEyeColor, BrokkerPlumageColor, BrokkerSnoutColor};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BrokkerColors {
	pub plumage: BrokkerPlumageColor,
	pub eyes: BrokkerEyeColor,
	pub snout: BrokkerSnoutColor,
	pub hair: ItemColor,
	pub clothing_default: ItemColor,
	pub clothing_material: ClothingMaterial,
	pub clothing: Vec<ClothingColor>,
}

impl Default for BrokkerColors {
	fn default() -> Self {
		Self {
			plumage: BrokkerPlumageColor::Olive,
			eyes: BrokkerEyeColor::Amber,
			snout: BrokkerSnoutColor::Horn,
			hair: ItemColor::Dark,
			clothing_default: ItemColor::Cool,
			clothing_material: ClothingMaterial::Cloth,
			clothing: Vec::new(),
		}
	}
}

impl BrokkerColors {
	pub fn color_for_slot(&self, slot: crate::CharacterPartSlot) -> bevy::prelude::Color {
		use crate::CharacterPartSlot::*;
		match slot {
			BodyMesh | HeadMesh | HeadRig | Hair => self.plumage.color(),
			EyeLeft | EyeRight => self.eyes.color(),
			Mouth => self.snout.color(),
			_ => self.plumage.color(),
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
pub struct BrokkerConfig {
	pub eye: EyeMesh,
	pub hair: HairMesh,
	pub clothing: Vec<ClothingMesh>,
	pub colors: BrokkerColors,
}

impl Default for BrokkerConfig {
	fn default() -> Self {
		Self::default_preview()
	}
}

impl BrokkerConfig {
	pub fn default_preview() -> Self {
		Self {
			eye: EyeMesh::Falcon,
			hair: HairMesh::None,
			clothing: Vec::new(),
			colors: BrokkerColors::default(),
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
			"brokker eye={} hair={} clothing={} plumage={} eyes={} snout_color={}",
			self.eye.label(),
			self.hair.label(),
			clothing,
			self.colors.plumage.label(),
			self.colors.eyes.label(),
			self.colors.snout.label(),
		)
	}

	pub fn sync_key(&self) -> String {
		format!("{self:?}")
	}
}

impl CharacterRecipe for BrokkerConfig {
	type Components = Brokker;

	fn components(&self) -> Self::Components {
		Brokker::from_config(self)
	}

	fn clothing_layers(&self) -> Vec<ClothingLayer> {
		crate::clothing_layers(
			self.clothing.iter().copied(),
			ClothingHost::LIBIRD,
			self.colors.clothing_material,
			|mesh| self.colors.clothing_color(mesh),
		)
	}
}
