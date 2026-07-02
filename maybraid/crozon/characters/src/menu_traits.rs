//! Menu option trait implementations for Crozon character types.

use bevy_math::Vec3;
use character_ui_menu::{AssetOption, IdentifiedAsset, LabelOption, ListValues, StringIdentified, SwatchOption, ThumbnailCamera};

use crate::{
	presets::{BuildPreset, GenderPreset},
	species::{
		braidman::BraidmanColor,
		brodler::{
			assets::HornMesh, BrodlerEyeColor, BrodlerHeadMesh, BrodlerHornColor, BrodlerSkinColor,
		},
		mygr::{MygrEyeColor, MygrHeadMesh, MygrMouthMesh, MygrSkinColor},
		dui::{DuiEyeMesh, DuiHeadMesh, DuiMouthMesh, DuiNoseMesh, DuiSkinColor},
		common::{
			BodyMesh, ClothingMesh, EarMesh, EyeMesh, HairMesh, HeadMesh, MouthMesh, NoseMesh,
		},
	},
	ConceptAnimation,
};

macro_rules! impl_menu_identity {
	($ty:ty) => {
		impl ListValues for $ty {
			fn values() -> &'static [Self] {
				Self::VALUES
			}
		}

		impl LabelOption for $ty {
			fn label(&self) -> &'static str {
				(*self).label()
			}
		}

		impl StringIdentified for $ty {
			fn id(&self) -> &'static str {
				(*self).label()
			}
		}
	};
}

impl_menu_identity!(GenderPreset);
impl_menu_identity!(BuildPreset);
impl_menu_identity!(BodyMesh);
impl_menu_identity!(HeadMesh);
impl_menu_identity!(BrodlerHeadMesh);
impl_menu_identity!(HornMesh);
impl_menu_identity!(EyeMesh);
impl_menu_identity!(NoseMesh);
impl_menu_identity!(MouthMesh);
impl_menu_identity!(EarMesh);
impl_menu_identity!(HairMesh);
impl_menu_identity!(ClothingMesh);
impl_menu_identity!(ConceptAnimation);
impl_menu_identity!(BraidmanColor);
impl_menu_identity!(BrodlerSkinColor);
impl_menu_identity!(BrodlerEyeColor);
impl_menu_identity!(BrodlerHornColor);
impl_menu_identity!(MygrHeadMesh);
impl_menu_identity!(MygrMouthMesh);
impl_menu_identity!(MygrSkinColor);
impl_menu_identity!(MygrEyeColor);
impl_menu_identity!(DuiHeadMesh);
impl_menu_identity!(DuiEyeMesh);
impl_menu_identity!(DuiNoseMesh);
impl_menu_identity!(DuiMouthMesh);
impl_menu_identity!(DuiSkinColor);

macro_rules! impl_asset_option {
	($ty:ty, $camera:expr) => {
		impl AssetOption for $ty {
			fn asset(&self) -> IdentifiedAsset {
				let path = (*self).path();
				IdentifiedAsset::new((*self).label(), (*self).label(), path.as_str())
					.with_thumbnail_camera($camera)
			}
		}
	};
}

const BODY_THUMBNAIL_CAMERA: ThumbnailCamera =
	ThumbnailCamera::new(Vec3::new(0.0, 0.8, 3.2), Vec3::new(0.0, 0.65, 0.0));
const HEAD_THUMBNAIL_CAMERA: ThumbnailCamera =
	ThumbnailCamera::new(Vec3::new(0.0, 0.05, 1.25), Vec3::new(0.0, 0.05, 0.0));
const FACE_FEATURE_THUMBNAIL_CAMERA: ThumbnailCamera =
	ThumbnailCamera::new(Vec3::new(0.0, 0.0, 0.55), Vec3::ZERO);
const EAR_THUMBNAIL_CAMERA: ThumbnailCamera =
	ThumbnailCamera::new(Vec3::new(0.45, 0.0, 0.65), Vec3::ZERO);
const CROWN_THUMBNAIL_CAMERA: ThumbnailCamera =
	ThumbnailCamera::new(Vec3::new(0.0, 0.2, 1.3), Vec3::new(0.0, 0.1, 0.0));
const CLOTHING_THUMBNAIL_CAMERA: ThumbnailCamera =
	ThumbnailCamera::new(Vec3::new(0.0, 0.75, 2.5), Vec3::new(0.0, 0.65, 0.0));

impl_asset_option!(BodyMesh, BODY_THUMBNAIL_CAMERA);
impl_asset_option!(HeadMesh, HEAD_THUMBNAIL_CAMERA);
impl_asset_option!(BrodlerHeadMesh, HEAD_THUMBNAIL_CAMERA);
impl_asset_option!(HornMesh, CROWN_THUMBNAIL_CAMERA);
impl_asset_option!(EyeMesh, FACE_FEATURE_THUMBNAIL_CAMERA);
impl_asset_option!(NoseMesh, FACE_FEATURE_THUMBNAIL_CAMERA);
impl_asset_option!(MouthMesh, FACE_FEATURE_THUMBNAIL_CAMERA);
impl_asset_option!(EarMesh, EAR_THUMBNAIL_CAMERA);
impl_asset_option!(ClothingMesh, CLOTHING_THUMBNAIL_CAMERA);
impl_asset_option!(MygrHeadMesh, HEAD_THUMBNAIL_CAMERA);
impl_asset_option!(MygrMouthMesh, FACE_FEATURE_THUMBNAIL_CAMERA);
impl_asset_option!(DuiHeadMesh, HEAD_THUMBNAIL_CAMERA);
impl_asset_option!(DuiEyeMesh, FACE_FEATURE_THUMBNAIL_CAMERA);
impl_asset_option!(DuiMouthMesh, FACE_FEATURE_THUMBNAIL_CAMERA);

impl AssetOption for DuiNoseMesh {
	fn asset(&self) -> IdentifiedAsset {
		match self {
			Self::None => IdentifiedAsset::new("none", "none", ""),
			Self::Tbar => {
				let path = (*self).path();
				IdentifiedAsset::new((*self).label(), (*self).label(), path.as_str())
					.with_thumbnail_camera(FACE_FEATURE_THUMBNAIL_CAMERA)
			}
		}
	}
}

impl AssetOption for HairMesh {
	fn asset(&self) -> IdentifiedAsset {
		let label = (*self).label();
		let path = (*self).path().map(|path| path.as_str()).unwrap_or("");
		IdentifiedAsset::new(label, label, path).with_thumbnail_camera(CROWN_THUMBNAIL_CAMERA)
	}
}

impl AssetOption for ConceptAnimation {
	fn asset(&self) -> IdentifiedAsset {
		IdentifiedAsset::new((*self).label(), (*self).label(), "")
	}
}

impl SwatchOption for BraidmanColor {
	fn color_hex(&self) -> &'static str {
		match self {
			Self::Natural => "#B88A6B",
			Self::Warm => "#DB9441",
			Self::Cool => "#7599B8",
			Self::Dark => "#2E2926",
			Self::Light => "#E0CCAE",
			Self::Red => "#B82E29",
			Self::Blue => "#2E4DC2",
			Self::Green => "#388547",
			Self::Gold => "#E0AD38",
		}
	}
}

impl SwatchOption for BrodlerSkinColor {
	fn color_hex(&self) -> &'static str {
		match self {
			Self::Crimson => "#941E1E",
			Self::Umber => "#4D3329",
			Self::Ochre => "#AD8529",
		}
	}
}

impl SwatchOption for BrodlerEyeColor {
	fn color_hex(&self) -> &'static str {
		match self {
			Self::Black => "#141419",
			Self::LightBlue => "#85B3D1",
			Self::Yellow => "#D1B847",
		}
	}
}

impl SwatchOption for BrodlerHornColor {
	fn color_hex(&self) -> &'static str {
		match self {
			Self::LightBrown => "#9E7A4D",
			Self::Yellow => "#C7A847",
		}
	}
}

impl SwatchOption for MygrSkinColor {
	fn color_hex(&self) -> &'static str {
		match self {
			Self::Ginger => "#C47A3A",
			Self::Charcoal => "#282624",
			Self::Silver => "#8A8F94",
			Self::Cream => "#E8DCC8",
			Self::Tawny => "#8B5E3C",
		}
	}
}

impl SwatchOption for MygrEyeColor {
	fn color_hex(&self) -> &'static str {
		match self {
			Self::Green => "#4A8C4F",
			Self::Amber => "#C9A227",
			Self::Blue => "#6BA3D1",
		}
	}
}

impl SwatchOption for DuiSkinColor {
	fn color_hex(&self) -> &'static str {
		match self {
			Self::Purple => "#7A6685",
			Self::DesertBrown => "#9C7A5C",
			Self::Blue => "#5A7A8C",
			Self::Gold => "#C4A052",
		}
	}
}
