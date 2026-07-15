//! Menu option trait implementations for Crozon character types.
//!
//! Identity/labeling impls are macro-generated; swatch colors delegate to the
//! species palette `color_hex()` definitions so hex values live next to their
//! sRGB counterparts.

use bevy_math::Vec3;
use character_ui_menu::{
	AssetOption, IdentifiedAsset, LabelOption, ListValues, StringIdentified, SwatchOption,
	ThumbnailCamera,
};

use crate::{
	presets::{BuildPreset, GenderPreset},
	species::{
		brodler::{
			assets::HornMesh, BrodlerEyeColor, BrodlerHeadMesh, BrodlerHornColor, BrodlerSkinColor,
		},
		brenal::{BrenalBodyMesh, BrenalHeadMesh, BrenalHornMesh, BrenalMouthMesh},
		caole::{CaoleBodyMesh, CaoleMouthMesh},
		brokker::{
			BrokkerEyeColor, BrokkerHeadMesh, BrokkerPlumageColor, BrokkerSnoutColor,
			BrokkerSnoutMesh,
		},
		chupri::{
			ChupriBeakColor, ChupriBeakMesh, ChupriEyeColor, ChupriHeadMesh, ChupriPlumageColor,
		},
		epiphant::{
			EpiphantBodyMesh, EpiphantColor, EpiphantEarMesh, EpiphantHeadMesh, EpiphantNoseMesh,
		},
		hars::{HarsBodyMesh, HarsMouthMesh},
		claber::{ClaberBodyMesh, ClaberColor, ClaberHeadMesh, ClaberHornMesh, ClaberMouthMesh},
		croconot::{CroconotBodyMesh, CroconotHeadMesh, CroconotHornMesh, CroconotMouthMesh},
		common::{BodyMesh, EarMesh, EyeMesh, HairMesh, HeadMesh, MouthMesh, NoseMesh},
		dui::{
			DuiEyeColor, DuiEyeMesh, DuiHeadMesh, DuiMouthColor, DuiMouthMesh, DuiNoseColor,
			DuiNoseMesh, DuiSkinColor,
		},
		kispar::{
			KisparBeakColor, KisparBeakMesh, KisparEyeColor, KisparHeadMesh, KisparPlumageColor,
		},
		tapp::{
			TappBeakColor, TappBeakMesh, TappEyeColor, TappHeadMesh, TappPlumageColor,
		},
		kaller::{
			KallerCrownColor, KallerEyeColor, KallerHeadMesh, KallerHornMesh, KallerPlumageColor,
			KallerSnoutColor, KallerSnoutMesh,
		},
		kappler::{
			KapplerBeakColor, KapplerBeakMesh, KapplerEyeColor, KapplerHeadMesh, KapplerPlumageColor,
		},
		lero::{
			LeroEyeColor, LeroHeadMesh, LeroMouthColor, LeroMouthMesh, LeroSkinColor,
			LeroSpineColor, LeroTailColor,
		},
		lidder::{LidderBeakColor, LidderBeakMesh, LidderEyeColor, LidderHeadMesh, LidderPlumageColor},
		mygr::{MygrEyeColor, MygrHeadMesh, MygrMouthMesh, MygrSkinColor},
		spibmom::{
			SpibmomCrownColor, SpibmomEarColor, SpibmomEyeColor, SpibmomHeadMesh,
			SpibmomMouthColor, SpibmomMouthMesh, SpibmomSkinColor, SpibmomSpineColor,
		},
		tipple::{
			TippleBeakColor, TippleBeakMesh, TippleEyeColor, TippleHeadMesh, TipplePlumageColor,
		},
		grener::GrenerBodyColor,
		mistler::MistlerBodyColor,
		thumplus::ThumplusBodyColor,
		topple::{
			ToppleBeakColor, ToppleBeakMesh, ToppleEyeColor, ToppleHeadMesh, TopplePlumageColor,
		},
		wumbus::{
			WumbusEarColor, WumbusEyeColor, WumbusHeadMesh, WumbusHornColor, WumbusHornMesh,
			WumbusMouthColor, WumbusMouthMesh, WumbusSkinColor, WumbusSpineColor,
		},
		ylter::{YilterBodyMesh, YilterMouthMesh},
		sonyak::{SonyakBodyMesh, SonyakMouthMesh},
	},
	ConceptAnimation,
};

macro_rules! impl_menu_identity {
	($($ty:ty),+ $(,)?) => {$(
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
	)+};
}

impl_menu_identity!(
	GenderPreset,
	BuildPreset,
	BodyMesh,
	HeadMesh,
	BrodlerHeadMesh,
	HornMesh,
	EyeMesh,
	NoseMesh,
	MouthMesh,
	EarMesh,
	HairMesh,
	ConceptAnimation,
	BrodlerSkinColor,
	BrodlerEyeColor,
	BrodlerHornColor,
	MygrHeadMesh,
	MygrMouthMesh,
	MygrSkinColor,
	MygrEyeColor,
	WumbusHeadMesh,
	WumbusMouthMesh,
	WumbusHornMesh,
	WumbusSkinColor,
	WumbusEyeColor,
	WumbusEarColor,
	WumbusMouthColor,
	WumbusHornColor,
	WumbusSpineColor,
	LeroHeadMesh,
	LeroMouthMesh,
	LeroMouthColor,
	LeroSkinColor,
	LeroEyeColor,
	LeroTailColor,
	LeroSpineColor,
	SpibmomHeadMesh,
	SpibmomMouthMesh,
	SpibmomSkinColor,
	SpibmomEyeColor,
	SpibmomEarColor,
	SpibmomMouthColor,
	SpibmomCrownColor,
	SpibmomSpineColor,
	DuiHeadMesh,
	DuiEyeMesh,
	DuiNoseMesh,
	DuiMouthMesh,
	DuiSkinColor,
	DuiEyeColor,
	DuiNoseColor,
	DuiMouthColor,
	LidderHeadMesh,
	LidderBeakMesh,
	LidderPlumageColor,
	LidderEyeColor,
	LidderBeakColor,
	ChupriHeadMesh,
	ChupriBeakMesh,
	ChupriPlumageColor,
	ChupriEyeColor,
	ChupriBeakColor,
	BrokkerHeadMesh,
	BrokkerSnoutMesh,
	BrokkerPlumageColor,
	BrokkerEyeColor,
	BrokkerSnoutColor,
	TippleHeadMesh,
	TippleBeakMesh,
	TipplePlumageColor,
	TippleEyeColor,
	TippleBeakColor,
	GrenerBodyColor,
	ThumplusBodyColor,
	MistlerBodyColor,
	ToppleHeadMesh,
	ToppleBeakMesh,
	TopplePlumageColor,
	ToppleEyeColor,
	ToppleBeakColor,
	KisparHeadMesh,
	KisparBeakMesh,
	KisparPlumageColor,
	KisparEyeColor,
	KisparBeakColor,
	TappHeadMesh,
	TappBeakMesh,
	TappPlumageColor,
	TappEyeColor,
	TappBeakColor,
	KallerHeadMesh,
	KallerSnoutMesh,
	KallerHornMesh,
	KallerPlumageColor,
	KallerEyeColor,
	KallerSnoutColor,
	KallerCrownColor,
	KapplerHeadMesh,
	KapplerBeakMesh,
	KapplerPlumageColor,
	KapplerEyeColor,
	KapplerBeakColor,
	BrenalBodyMesh,
	BrenalHeadMesh,
	BrenalMouthMesh,
	BrenalHornMesh,
	CaoleBodyMesh,
	CaoleMouthMesh,
	EpiphantBodyMesh,
	EpiphantHeadMesh,
	EpiphantEarMesh,
	EpiphantNoseMesh,
	EpiphantColor,
	HarsBodyMesh,
	HarsMouthMesh,
	YilterBodyMesh,
	YilterMouthMesh,
	SonyakBodyMesh,
	SonyakMouthMesh,
	ClaberBodyMesh,
	ClaberHeadMesh,
	ClaberMouthMesh,
	ClaberHornMesh,
	ClaberColor,
	CroconotBodyMesh,
	CroconotHeadMesh,
	CroconotMouthMesh,
	CroconotHornMesh,
);

macro_rules! impl_swatch_option {
	($($ty:ty),+ $(,)?) => {$(
		impl SwatchOption for $ty {
			fn color_hex(&self) -> &'static str {
				(*self).color_hex()
			}
		}
	)+};
}

impl_swatch_option!(
	BrodlerSkinColor,
	BrodlerEyeColor,
	BrodlerHornColor,
	MygrSkinColor,
	MygrEyeColor,
	DuiSkinColor,
	DuiEyeColor,
	DuiNoseColor,
	DuiMouthColor,
	LidderPlumageColor,
	LidderEyeColor,
	LidderBeakColor,
	ChupriPlumageColor,
	ChupriEyeColor,
	ChupriBeakColor,
	BrokkerPlumageColor,
	BrokkerEyeColor,
	BrokkerSnoutColor,
	TipplePlumageColor,
	TippleEyeColor,
	TippleBeakColor,
	GrenerBodyColor,
	ThumplusBodyColor,
	MistlerBodyColor,
	TopplePlumageColor,
	ToppleEyeColor,
	ToppleBeakColor,
	KisparPlumageColor,
	KisparEyeColor,
	KisparBeakColor,
	TappPlumageColor,
	TappEyeColor,
	TappBeakColor,
	KallerPlumageColor,
	KallerEyeColor,
	KallerSnoutColor,
	KallerCrownColor,
	KapplerPlumageColor,
	KapplerEyeColor,
	KapplerBeakColor,
	WumbusSkinColor,
	WumbusEyeColor,
	WumbusEarColor,
	WumbusMouthColor,
	WumbusHornColor,
	WumbusSpineColor,
	LeroSkinColor,
	LeroEyeColor,
	LeroMouthColor,
	LeroTailColor,
	LeroSpineColor,
	SpibmomSkinColor,
	SpibmomEyeColor,
	SpibmomEarColor,
	SpibmomMouthColor,
	SpibmomCrownColor,
	SpibmomSpineColor,
	ClaberColor,
	EpiphantColor,
);

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

impl_asset_option!(BodyMesh, BODY_THUMBNAIL_CAMERA);
impl_asset_option!(HeadMesh, HEAD_THUMBNAIL_CAMERA);
impl_asset_option!(BrodlerHeadMesh, HEAD_THUMBNAIL_CAMERA);
impl_asset_option!(HornMesh, CROWN_THUMBNAIL_CAMERA);
impl_asset_option!(EyeMesh, FACE_FEATURE_THUMBNAIL_CAMERA);
impl_asset_option!(NoseMesh, FACE_FEATURE_THUMBNAIL_CAMERA);
impl_asset_option!(MouthMesh, FACE_FEATURE_THUMBNAIL_CAMERA);
impl_asset_option!(EarMesh, EAR_THUMBNAIL_CAMERA);
impl_asset_option!(MygrHeadMesh, HEAD_THUMBNAIL_CAMERA);
impl_asset_option!(MygrMouthMesh, FACE_FEATURE_THUMBNAIL_CAMERA);
impl_asset_option!(WumbusHeadMesh, HEAD_THUMBNAIL_CAMERA);
impl_asset_option!(WumbusMouthMesh, FACE_FEATURE_THUMBNAIL_CAMERA);
impl_asset_option!(LeroHeadMesh, HEAD_THUMBNAIL_CAMERA);
impl_asset_option!(LeroMouthMesh, FACE_FEATURE_THUMBNAIL_CAMERA);
impl_asset_option!(SpibmomHeadMesh, HEAD_THUMBNAIL_CAMERA);
impl_asset_option!(SpibmomMouthMesh, FACE_FEATURE_THUMBNAIL_CAMERA);
impl_asset_option!(DuiHeadMesh, HEAD_THUMBNAIL_CAMERA);
impl_asset_option!(LidderHeadMesh, HEAD_THUMBNAIL_CAMERA);
impl_asset_option!(LidderBeakMesh, FACE_FEATURE_THUMBNAIL_CAMERA);
impl_asset_option!(ChupriHeadMesh, HEAD_THUMBNAIL_CAMERA);
impl_asset_option!(ChupriBeakMesh, FACE_FEATURE_THUMBNAIL_CAMERA);
impl_asset_option!(BrokkerHeadMesh, HEAD_THUMBNAIL_CAMERA);
impl_asset_option!(BrokkerSnoutMesh, FACE_FEATURE_THUMBNAIL_CAMERA);
impl_asset_option!(TippleHeadMesh, HEAD_THUMBNAIL_CAMERA);
impl_asset_option!(TippleBeakMesh, FACE_FEATURE_THUMBNAIL_CAMERA);
impl_asset_option!(ToppleHeadMesh, HEAD_THUMBNAIL_CAMERA);
impl_asset_option!(ToppleBeakMesh, FACE_FEATURE_THUMBNAIL_CAMERA);
impl_asset_option!(KisparHeadMesh, HEAD_THUMBNAIL_CAMERA);
impl_asset_option!(KisparBeakMesh, FACE_FEATURE_THUMBNAIL_CAMERA);
impl_asset_option!(TappHeadMesh, HEAD_THUMBNAIL_CAMERA);
impl_asset_option!(TappBeakMesh, FACE_FEATURE_THUMBNAIL_CAMERA);
impl_asset_option!(KallerHeadMesh, HEAD_THUMBNAIL_CAMERA);
impl_asset_option!(KallerSnoutMesh, FACE_FEATURE_THUMBNAIL_CAMERA);
impl_asset_option!(KallerHornMesh, CROWN_THUMBNAIL_CAMERA);
impl_asset_option!(KapplerHeadMesh, HEAD_THUMBNAIL_CAMERA);
impl_asset_option!(KapplerBeakMesh, FACE_FEATURE_THUMBNAIL_CAMERA);
impl_asset_option!(BrenalBodyMesh, BODY_THUMBNAIL_CAMERA);
impl_asset_option!(BrenalHeadMesh, HEAD_THUMBNAIL_CAMERA);
impl_asset_option!(BrenalMouthMesh, FACE_FEATURE_THUMBNAIL_CAMERA);
impl_asset_option!(CaoleBodyMesh, BODY_THUMBNAIL_CAMERA);
impl_asset_option!(CaoleMouthMesh, FACE_FEATURE_THUMBNAIL_CAMERA);
impl_asset_option!(EpiphantBodyMesh, BODY_THUMBNAIL_CAMERA);
impl_asset_option!(EpiphantHeadMesh, HEAD_THUMBNAIL_CAMERA);
impl_asset_option!(EpiphantEarMesh, EAR_THUMBNAIL_CAMERA);
impl_asset_option!(EpiphantNoseMesh, FACE_FEATURE_THUMBNAIL_CAMERA);
impl_asset_option!(HarsBodyMesh, BODY_THUMBNAIL_CAMERA);
impl_asset_option!(HarsMouthMesh, FACE_FEATURE_THUMBNAIL_CAMERA);
impl_asset_option!(YilterBodyMesh, BODY_THUMBNAIL_CAMERA);
impl_asset_option!(YilterMouthMesh, FACE_FEATURE_THUMBNAIL_CAMERA);
impl_asset_option!(SonyakBodyMesh, BODY_THUMBNAIL_CAMERA);
impl_asset_option!(SonyakMouthMesh, FACE_FEATURE_THUMBNAIL_CAMERA);
impl_asset_option!(ClaberBodyMesh, BODY_THUMBNAIL_CAMERA);
impl_asset_option!(ClaberHeadMesh, HEAD_THUMBNAIL_CAMERA);
impl_asset_option!(ClaberMouthMesh, FACE_FEATURE_THUMBNAIL_CAMERA);
impl_asset_option!(CroconotBodyMesh, BODY_THUMBNAIL_CAMERA);
impl_asset_option!(CroconotHeadMesh, HEAD_THUMBNAIL_CAMERA);
impl_asset_option!(CroconotMouthMesh, FACE_FEATURE_THUMBNAIL_CAMERA);
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

impl AssetOption for BrenalHornMesh {
	fn asset(&self) -> IdentifiedAsset {
		match self {
			Self::None => IdentifiedAsset::new("none", "none", ""),
			Self::HarrowedCrown => {
				let path = (*self).path();
				IdentifiedAsset::new((*self).label(), (*self).label(), path.as_str())
					.with_thumbnail_camera(CROWN_THUMBNAIL_CAMERA)
			}
		}
	}
}

impl AssetOption for ClaberHornMesh {
	fn asset(&self) -> IdentifiedAsset {
		match self {
			Self::None => IdentifiedAsset::new("none", "none", ""),
			Self::HarrowedCrown => {
				let path = (*self).path();
				IdentifiedAsset::new((*self).label(), (*self).label(), path.as_str())
					.with_thumbnail_camera(CROWN_THUMBNAIL_CAMERA)
			}
		}
	}
}

impl AssetOption for CroconotHornMesh {
	fn asset(&self) -> IdentifiedAsset {
		match self {
			Self::None => IdentifiedAsset::new("none", "none", ""),
			Self::HarrowedCrown => {
				let path = (*self).path();
				IdentifiedAsset::new((*self).label(), (*self).label(), path.as_str())
					.with_thumbnail_camera(CROWN_THUMBNAIL_CAMERA)
			}
		}
	}
}

impl AssetOption for WumbusHornMesh {
	fn asset(&self) -> IdentifiedAsset {
		match self {
			Self::None => IdentifiedAsset::new("none", "none", ""),
			Self::HarrowedCrown => {
				let path = (*self).path();
				IdentifiedAsset::new((*self).label(), (*self).label(), path.as_str())
					.with_thumbnail_camera(CROWN_THUMBNAIL_CAMERA)
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
