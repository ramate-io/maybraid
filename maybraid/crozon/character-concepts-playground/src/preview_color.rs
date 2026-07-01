//! Shared preview color for material and thumbnail tinting.

use bevy::prelude::*;
use crozon_characters::species::{
	braidman::BraidmanColor,
	brodler::{BrodlerEyeColor, BrodlerSkinColor},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PreviewColor {
	Braidman(BraidmanColor),
	BrodlerSkin(BrodlerSkinColor),
	BrodlerEye(BrodlerEyeColor),
}

impl PreviewColor {
	pub fn bevy_color(self) -> Color {
		match self {
			Self::Braidman(color) => color.color(),
			Self::BrodlerSkin(color) => color.color(),
			Self::BrodlerEye(color) => color.color(),
		}
	}
}
