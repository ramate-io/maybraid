use crozon_characters::{
	species::{
		braidman::BraidmanColor,
		brodler::{assets::HornMesh, BrodlerEyeColor, BrodlerHeadMesh, BrodlerHornColor, BrodlerSkinColor},
		mygr::{MygrEyeColor, MygrHeadMesh, MygrMouthMesh, MygrSkinColor},
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
