use crozon_characters::{
	species::{
		braidman::BraidmanColor,
		brodler::{assets::HornMesh, BrodlerEyeColor, BrodlerHeadMesh, BrodlerHornColor, BrodlerSkinColor},
		mygr::{MygrEyeColor, MygrHeadMesh, MygrMouthMesh, MygrSkinColor},
		wumbus::{
			WumbusEarColor, WumbusEyeColor, WumbusHeadMesh, WumbusHornColor, WumbusMouthColor,
			WumbusMouthMesh, WumbusSkinColor, WumbusSpineColor,
		},
		lero::{
			LeroEyeColor, LeroHeadMesh, LeroMouthMesh, LeroSkinColor, LeroSpineColor, LeroTailColor,
		},
		dui::{DuiEyeMesh, DuiHeadMesh, DuiMouthMesh, DuiMouthColor, DuiSkinColor},
		common::{BodyMesh, EarMesh, EyeMesh, HairMesh, HeadMesh, MouthMesh, NoseMesh},
	},
	ConceptAnimation,
};

use crate::{
	event::{AssetValue, SwatchValue},
	fields::{AssetFieldValue, SwatchFieldValue},
};

impl AssetFieldValue for BodyMesh {
	fn to_asset_value(value: Self) -> AssetValue {
		AssetValue::Body(value)
	}
}

impl AssetFieldValue for HeadMesh {
	fn to_asset_value(value: Self) -> AssetValue {
		AssetValue::Head(value)
	}
}

impl AssetFieldValue for BrodlerHeadMesh {
	fn to_asset_value(value: Self) -> AssetValue {
		AssetValue::BrodlerHead(value)
	}
}

impl AssetFieldValue for HornMesh {
	fn to_asset_value(value: Self) -> AssetValue {
		AssetValue::Horns(value)
	}
}

impl AssetFieldValue for MygrHeadMesh {
	fn to_asset_value(value: Self) -> AssetValue {
		AssetValue::MygrHead(value)
	}
}

impl AssetFieldValue for MygrMouthMesh {
	fn to_asset_value(value: Self) -> AssetValue {
		AssetValue::MygrMouth(value)
	}
}

impl AssetFieldValue for WumbusHeadMesh {
	fn to_asset_value(value: Self) -> AssetValue {
		AssetValue::WumbusHead(value)
	}
}

impl AssetFieldValue for WumbusMouthMesh {
	fn to_asset_value(value: Self) -> AssetValue {
		AssetValue::WumbusMouth(value)
	}
}

impl AssetFieldValue for DuiHeadMesh {
	fn to_asset_value(value: Self) -> AssetValue {
		AssetValue::DuiHead(value)
	}
}

impl AssetFieldValue for DuiEyeMesh {
	fn to_asset_value(value: Self) -> AssetValue {
		AssetValue::DuiEye(value)
	}
}

impl AssetFieldValue for DuiMouthMesh {
	fn to_asset_value(value: Self) -> AssetValue {
		AssetValue::DuiMouth(value)
	}
}

impl AssetFieldValue for EyeMesh {
	fn to_asset_value(value: Self) -> AssetValue {
		AssetValue::Eye(value)
	}
}

impl AssetFieldValue for NoseMesh {
	fn to_asset_value(value: Self) -> AssetValue {
		AssetValue::Nose(value)
	}
}

impl AssetFieldValue for MouthMesh {
	fn to_asset_value(value: Self) -> AssetValue {
		AssetValue::Mouth(value)
	}
}

impl AssetFieldValue for EarMesh {
	fn to_asset_value(value: Self) -> AssetValue {
		AssetValue::Ear(value)
	}
}

impl AssetFieldValue for HairMesh {
	fn to_asset_value(value: Self) -> AssetValue {
		AssetValue::Hair(value)
	}
}

impl AssetFieldValue for ConceptAnimation {
	fn to_asset_value(value: Self) -> AssetValue {
		AssetValue::Animation(value)
	}
}

impl SwatchFieldValue for BraidmanColor {
	fn to_swatch_value(value: Self) -> SwatchValue {
		SwatchValue::Braidman(value)
	}
}

impl SwatchFieldValue for BrodlerSkinColor {
	fn to_swatch_value(value: Self) -> SwatchValue {
		SwatchValue::BrodlerSkin(value)
	}
}

impl SwatchFieldValue for BrodlerEyeColor {
	fn to_swatch_value(value: Self) -> SwatchValue {
		SwatchValue::BrodlerEye(value)
	}
}

impl SwatchFieldValue for BrodlerHornColor {
	fn to_swatch_value(value: Self) -> SwatchValue {
		SwatchValue::BrodlerHorn(value)
	}
}

impl SwatchFieldValue for MygrSkinColor {
	fn to_swatch_value(value: Self) -> SwatchValue {
		SwatchValue::MygrSkin(value)
	}
}

impl SwatchFieldValue for MygrEyeColor {
	fn to_swatch_value(value: Self) -> SwatchValue {
		SwatchValue::MygrEye(value)
	}
}

impl SwatchFieldValue for WumbusSkinColor {
	fn to_swatch_value(value: Self) -> SwatchValue {
		SwatchValue::WumbusSkin(value)
	}
}

impl SwatchFieldValue for WumbusEyeColor {
	fn to_swatch_value(value: Self) -> SwatchValue {
		SwatchValue::WumbusEye(value)
	}
}

impl SwatchFieldValue for WumbusEarColor {
	fn to_swatch_value(value: Self) -> SwatchValue {
		SwatchValue::WumbusEar(value)
	}
}

impl SwatchFieldValue for WumbusMouthColor {
	fn to_swatch_value(value: Self) -> SwatchValue {
		SwatchValue::WumbusMouth(value)
	}
}

impl SwatchFieldValue for WumbusHornColor {
	fn to_swatch_value(value: Self) -> SwatchValue {
		SwatchValue::WumbusHorn(value)
	}
}

impl SwatchFieldValue for WumbusSpineColor {
	fn to_swatch_value(value: Self) -> SwatchValue {
		SwatchValue::WumbusSpine(value)
	}
}

impl AssetFieldValue for LeroHeadMesh {
	fn to_asset_value(value: Self) -> AssetValue {
		AssetValue::LeroHead(value)
	}
}

impl AssetFieldValue for LeroMouthMesh {
	fn to_asset_value(value: Self) -> AssetValue {
		AssetValue::LeroMouth(value)
	}
}

impl SwatchFieldValue for LeroSkinColor {
	fn to_swatch_value(value: Self) -> SwatchValue {
		SwatchValue::LeroSkin(value)
	}
}

impl SwatchFieldValue for LeroEyeColor {
	fn to_swatch_value(value: Self) -> SwatchValue {
		SwatchValue::LeroEye(value)
	}
}

impl SwatchFieldValue for LeroTailColor {
	fn to_swatch_value(value: Self) -> SwatchValue {
		SwatchValue::LeroTail(value)
	}
}

impl SwatchFieldValue for LeroSpineColor {
	fn to_swatch_value(value: Self) -> SwatchValue {
		SwatchValue::LeroSpine(value)
	}
}

impl SwatchFieldValue for DuiSkinColor {
	fn to_swatch_value(value: Self) -> SwatchValue {
		SwatchValue::DuiSkin(value)
	}
}

impl SwatchFieldValue for DuiMouthColor {
	fn to_swatch_value(value: Self) -> SwatchValue {
		SwatchValue::DuiMouth(value)
	}
}
