//! Crozon-specific typed character menus.

pub mod character;
pub mod characters;
pub mod event;
pub mod focus;

pub use character::{CharacterMenu, ConceptSpecies, SectionOpenState, SpeciesMenu};
pub use characters::{braidman::BraidmanMenu, brodler::BrodlerMenu};
pub use event::{AssetValue, CharacterField, MenuEvent, SectionId, SwatchValue};
pub use focus::BODY_FOCUS;

use bevy_math::Vec3;
use character_ui_menu::{IdentifiedAsset, ThumbnailCamera};
use crozon_characters::{
	presets::{BuildPreset, GenderPreset},
	species::{
		braidman::BraidmanColor,
		brodler::{
			assets::HornMesh, BrodlerEyeColor, BrodlerHeadMesh, BrodlerHornColor, BrodlerSkinColor,
		},
		common::{
			BodyMesh, ClothingMesh, EarMesh, EyeMesh, HairMesh, HeadMesh, MouthMesh, NoseMesh,
		},
	},
	ConceptAnimation,
};

/// Types with a fixed list of selectable variants.
pub trait ListValues: Copy + PartialEq + 'static {
	fn values() -> &'static [Self];
}

/// Stable string id for persistence and renderer keys.
pub trait StringIdentified {
	fn id(&self) -> &'static str;
}

/// Human-readable option label.
pub trait LabelOption {
	fn label(&self) -> &'static str;
}

/// Color swatch option contract.
pub trait SwatchOption: LabelOption {
	fn color_hex(&self) -> &'static str;
}

/// Asset-backed option contract.
pub trait AssetOption: LabelOption {
	fn asset(&self) -> IdentifiedAsset;
}

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

pub(crate) fn cycle_value<T: ListValues>(value: T, delta: i32) -> T {
	let values = T::values();
	let current = values.iter().position(|candidate| *candidate == value).unwrap_or(0);
	let next = (current as i32 + delta).rem_euclid(values.len() as i32) as usize;
	values[next]
}

#[cfg(test)]
mod tests;
