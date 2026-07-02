//! Shared preview color for material and thumbnail tinting.

use bevy::prelude::*;
use crozon_characters::species::{
	braidman::BraidmanColor,
	brodler::{BrodlerEyeColor, BrodlerHornColor, BrodlerSkinColor},
	mygr::{MygrEyeColor, MygrSkinColor},
	dui::{DuiEyeColor, DuiMouthColor, DuiNoseColor, DuiSkinColor},
	wumbus::{
		WumbusEarColor, WumbusEyeColor, WumbusHornColor, WumbusMouthColor, WumbusSkinColor,
	},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PreviewColor {
	Braidman(BraidmanColor),
	BrodlerSkin(BrodlerSkinColor),
	BrodlerEye(BrodlerEyeColor),
	BrodlerHorn(BrodlerHornColor),
	MygrSkin(MygrSkinColor),
	MygrEye(MygrEyeColor),
	DuiSkin(DuiSkinColor),
	DuiEye(DuiEyeColor),
	DuiNose(DuiNoseColor),
	DuiMouth(DuiMouthColor),
	WumbusSkin(WumbusSkinColor),
	WumbusEye(WumbusEyeColor),
	WumbusEar(WumbusEarColor),
	WumbusMouth(WumbusMouthColor),
	WumbusHorn(WumbusHornColor),
}

impl PreviewColor {
	pub fn bevy_color(self) -> Color {
		match self {
			Self::Braidman(color) => color.color(),
			Self::BrodlerSkin(color) => color.color(),
			Self::BrodlerEye(color) => color.color(),
			Self::BrodlerHorn(color) => color.color(),
			Self::MygrSkin(color) => color.color(),
			Self::MygrEye(color) => color.color(),
			Self::DuiSkin(color) => color.color(),
			Self::DuiEye(color) => color.color(),
			Self::DuiNose(color) => color.color(),
			Self::DuiMouth(color) => color.color(),
			Self::WumbusSkin(color) => color.color(),
			Self::WumbusEye(color) => color.color(),
			Self::WumbusEar(color) => color.color(),
			Self::WumbusMouth(color) => color.color(),
			Self::WumbusHorn(color) => color.color(),
		}
	}
}
