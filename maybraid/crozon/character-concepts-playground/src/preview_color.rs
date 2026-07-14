//! Shared preview color for material and thumbnail tinting.

use bevy::prelude::*;
use crozon_character_items::ItemColor;
use crozon_characters::species::{
	brodler::{BrodlerEyeColor, BrodlerHornColor, BrodlerSkinColor},
	claber::ClaberColor,
	dui::{DuiEyeColor, DuiMouthColor, DuiNoseColor, DuiSkinColor},
	lero::{LeroEyeColor, LeroMouthColor, LeroSkinColor, LeroSpineColor, LeroTailColor},
	mygr::{MygrEyeColor, MygrSkinColor},
	spibmom::{
		SpibmomCrownColor, SpibmomEarColor, SpibmomEyeColor, SpibmomMouthColor, SpibmomSkinColor,
		SpibmomSpineColor,
	},
	wumbus::{
		WumbusEarColor, WumbusEyeColor, WumbusHornColor, WumbusMouthColor, WumbusSkinColor,
		WumbusSpineColor,
	},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PreviewColor {
	/// Shared item palette: hair everywhere, plus Braidman body/eye/mouth.
	Item(ItemColor),
	BrodlerSkin(BrodlerSkinColor),
	BrodlerEye(BrodlerEyeColor),
	BrodlerHorn(BrodlerHornColor),
	MygrSkin(MygrSkinColor),
	MygrEye(MygrEyeColor),
	DuiSkin(DuiSkinColor),
	DuiEye(DuiEyeColor),
	DuiNose(DuiNoseColor),
	DuiMouth(DuiMouthColor),
	Claber(ClaberColor),
	WumbusSkin(WumbusSkinColor),
	WumbusEye(WumbusEyeColor),
	WumbusEar(WumbusEarColor),
	WumbusMouth(WumbusMouthColor),
	WumbusHorn(WumbusHornColor),
	WumbusSpine(WumbusSpineColor),
	LeroSkin(LeroSkinColor),
	LeroEye(LeroEyeColor),
	LeroMouth(LeroMouthColor),
	LeroTail(LeroTailColor),
	LeroSpine(LeroSpineColor),
	SpibmomSkin(SpibmomSkinColor),
	SpibmomEye(SpibmomEyeColor),
	SpibmomEar(SpibmomEarColor),
	SpibmomMouth(SpibmomMouthColor),
	SpibmomCrown(SpibmomCrownColor),
	SpibmomSpine(SpibmomSpineColor),
}

impl PreviewColor {
	pub fn bevy_color(self) -> Color {
		match self {
			Self::Item(color) => color.color(),
			Self::BrodlerSkin(color) => color.color(),
			Self::BrodlerEye(color) => color.color(),
			Self::BrodlerHorn(color) => color.color(),
			Self::MygrSkin(color) => color.color(),
			Self::MygrEye(color) => color.color(),
			Self::DuiSkin(color) => color.color(),
			Self::DuiEye(color) => color.color(),
			Self::DuiNose(color) => color.color(),
			Self::DuiMouth(color) => color.color(),
			Self::Claber(color) => color.color(),
			Self::WumbusSkin(color) => color.color(),
			Self::WumbusEye(color) => color.color(),
			Self::WumbusEar(color) => color.color(),
			Self::WumbusMouth(color) => color.color(),
			Self::WumbusHorn(color) => color.color(),
			Self::WumbusSpine(color) => color.color(),
			Self::LeroSkin(color) => color.color(),
			Self::LeroEye(color) => color.color(),
			Self::LeroMouth(color) => color.color(),
			Self::LeroTail(color) => color.color(),
			Self::LeroSpine(color) => color.color(),
			Self::SpibmomSkin(color) => color.color(),
			Self::SpibmomEye(color) => color.color(),
			Self::SpibmomEar(color) => color.color(),
			Self::SpibmomMouth(color) => color.color(),
			Self::SpibmomCrown(color) => color.color(),
			Self::SpibmomSpine(color) => color.color(),
		}
	}
}
